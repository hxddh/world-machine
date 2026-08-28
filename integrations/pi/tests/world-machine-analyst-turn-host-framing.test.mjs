import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";
import {
  AnalystTurnHostInputError,
  runAnalystTurnHost,
} from "../world-machine-analyst-turn-host.mjs";

class FramingSession {
  constructor() {
    this.prompts = [];
    this.probes = [];
    this.shutdownCount = 0;
  }

  async probe(options) {
    this.probes.push(options);
  }

  async prompt(prompt, options) {
    this.prompts.push({ prompt, options });
    return {
      requestId: `turn-${this.prompts.length}`,
      text: `answer-${this.prompts.length}`,
      toolCalls: [],
      extensionErrors: [],
    };
  }

  async shutdown() {
    this.shutdownCount += 1;
  }
}

function createHarness(maxInputRecordBytes) {
  const stdin = new PassThrough();
  const stdout = new PassThrough();
  const signalSource = new EventEmitter();
  const session = new FramingSession();
  let output = "";
  stdout.setEncoding("utf8");
  stdout.on("data", (chunk) => {
    output += chunk;
  });
  const promise = runAnalystTurnHost({
    stdin,
    stdout,
    argv: ["left.world", "right.world"],
    env: {},
    signalSource,
    maxInputRecordBytes,
    sessionFactory: () => session,
  });
  return { stdin, promise, session, output: () => output };
}

function exactSizedAskRequest(byteLength, id = "ask-1") {
  const empty = JSON.stringify({ id, op: "ask", prompt: "" });
  const fillerBytes = byteLength - Buffer.byteLength(empty);
  assert.ok(fillerBytes >= 1, "test record limit must fit a non-empty ask request");
  const record = JSON.stringify({ id, op: "ask", prompt: "x".repeat(fillerBytes) });
  assert.equal(Buffer.byteLength(record), byteLength);
  return record;
}

function responses(output) {
  return output.trim().length === 0
    ? []
    : output.trim().split("\n").map((line) => JSON.parse(line));
}

async function writeAndFinish(harness, chunks) {
  for (const chunk of chunks) harness.stdin.write(chunk);
  harness.stdin.end();
  await harness.promise;
}

test("turn-host stdin accepts an exact-limit LF request", async () => {
  const max = 96;
  const harness = createHarness(max);
  await writeAndFinish(harness, [`${exactSizedAskRequest(max)}\n`]);
  assert.equal(harness.session.prompts.length, 1);
  assert.equal(responses(harness.output())[0].id, "ask-1");
  assert.equal(harness.session.shutdownCount, 1);
});

test("turn-host stdin accepts an exact-limit CRLF request", async () => {
  const max = 96;
  const harness = createHarness(max);
  await writeAndFinish(harness, [`${exactSizedAskRequest(max)}\r\n`]);
  assert.equal(harness.session.prompts.length, 1);
  assert.equal(responses(harness.output())[0].type, "result");
});

test("turn-host stdin preserves exact-limit EOF-tail compatibility", async () => {
  const max = 96;
  const harness = createHarness(max);
  await writeAndFinish(harness, [exactSizedAskRequest(max)]);
  assert.equal(harness.session.prompts.length, 1);
  assert.equal(responses(harness.output())[0].id, "ask-1");
});

test("turn-host stdin accepts a request split across one-byte chunks", async () => {
  const max = 128;
  const harness = createHarness(max);
  const record = Buffer.from(`${JSON.stringify({ id: "probe-1", op: "probe" })}\n`);
  await writeAndFinish(harness, [...record].map((byte) => Buffer.from([byte])));
  assert.equal(harness.session.probes.length, 1);
  assert.equal(responses(harness.output())[0].id, "probe-1");
});

test("turn-host stdin preserves multiple request order from one chunk", async () => {
  const harness = createHarness(128);
  const first = JSON.stringify({ id: "probe-1", op: "probe" });
  const second = JSON.stringify({ id: "probe-2", op: "probe" });
  await writeAndFinish(harness, [`${first}\n${second}\n`]);
  assert.equal(harness.session.probes.length, 2);
  assert.deepEqual(responses(harness.output()).map((response) => response.id), ["probe-1", "probe-2"]);
});

test("turn-host stdin continues to ignore empty lines", async () => {
  const harness = createHarness(128);
  const probe = JSON.stringify({ id: "probe-1", op: "probe" });
  await writeAndFinish(harness, [`\n\r\n${probe}\n\n`]);
  assert.equal(harness.session.probes.length, 1);
  assert.equal(responses(harness.output()).length, 1);
});

test("turn-host stdin rejects newline-terminated overflow before parsing or handling", async () => {
  const max = 32;
  const harness = createHarness(max);
  harness.stdin.end(`${"x".repeat(max + 1)}\n`);
  await assert.rejects(
    harness.promise,
    (error) => error instanceof AnalystTurnHostInputError && /record exceeded/.test(error.message),
  );
  assert.equal(harness.session.prompts.length, 0);
  assert.equal(harness.session.probes.length, 0);
  assert.equal(harness.session.shutdownCount, 1);
  assert.equal(harness.output(), "");
});

test("turn-host stdin rejects no-newline overflow promptly without waiting for EOF", async () => {
  const max = 32;
  const harness = createHarness(max);
  harness.stdin.write(Buffer.alloc(max + 2, 0x78));
  try {
    await Promise.race([
      assert.rejects(harness.promise, /record exceeded/),
      new Promise((_, reject) => setTimeout(() => reject(new Error("framing overflow did not fail promptly")), 250)),
    ]);
  } finally {
    harness.stdin.destroy();
  }
  assert.equal(harness.session.shutdownCount, 1);
});

test("turn-host stdin rejects an oversized EOF tail", async () => {
  const max = 32;
  const harness = createHarness(max);
  harness.stdin.end(Buffer.alloc(max + 2, 0x78));
  await assert.rejects(harness.promise, /record exceeded/);
  assert.equal(harness.session.shutdownCount, 1);
});

test("turn-host stdin keeps a complete first request before a later oversized request in the same chunk", async () => {
  const max = 48;
  const harness = createHarness(max);
  const first = JSON.stringify({ id: "probe-1", op: "probe" });
  harness.stdin.end(`${first}\n${"x".repeat(max + 1)}\n`);
  await assert.rejects(harness.promise, /record exceeded/);
  assert.equal(harness.session.probes.length, 1);
  assert.equal(harness.session.prompts.length, 0);
  assert.equal(harness.session.shutdownCount, 1);
  const output = responses(harness.output());
  assert.equal(output.length, 1);
  assert.equal(output[0].id, "probe-1");
  assert.equal(output[0].type, "ready");
});
