import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PassThrough, Readable } from "node:stream";
import test from "node:test";
import {
  AnalystTurnHostInputError,
  readAnalystTurnLines,
  runAnalystTurnHost,
} from "../world-machine-analyst-turn-host.mjs";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function exactSizedAskRequest(byteLength, id = "ask-1") {
  const empty = JSON.stringify({ id, op: "ask", prompt: "" });
  const fillerBytes = byteLength - Buffer.byteLength(empty);
  assert.ok(fillerBytes >= 1, "test record limit must fit a non-empty ask request");
  const record = JSON.stringify({ id, op: "ask", prompt: "x".repeat(fillerBytes) });
  assert.equal(Buffer.byteLength(record), byteLength);
  return record;
}

async function collectLines(chunks, maxRecordBytes) {
  const lines = [];
  for await (const line of readAnalystTurnLines(Readable.from(chunks), maxRecordBytes)) {
    lines.push(line);
  }
  return lines;
}

test("turn-host stdin accepts an exact-limit LF request", async () => {
  const max = 96;
  const record = exactSizedAskRequest(max);
  assert.deepEqual(await collectLines([`${record}\n`], max), [record]);
});

test("turn-host stdin accepts an exact-limit CRLF request", async () => {
  const max = 96;
  const record = exactSizedAskRequest(max);
  assert.deepEqual(await collectLines([`${record}\r\n`], max), [record]);
});

test("turn-host stdin preserves exact-limit EOF-tail compatibility", async () => {
  const max = 96;
  const record = exactSizedAskRequest(max);
  assert.deepEqual(await collectLines([record], max), [record]);
  assert.deepEqual(await collectLines([`${record}\r`], max), [record]);
});

test("turn-host stdin accepts a request split across one-byte chunks", async () => {
  const max = 128;
  const record = JSON.stringify({ id: "probe-1", op: "probe" });
  const bytes = Buffer.from(`${record}\n`);
  assert.deepEqual(
    await collectLines([...bytes].map((byte) => Buffer.from([byte])), max),
    [record],
  );
});

test("turn-host stdin preserves multiple request order from one chunk", async () => {
  const first = JSON.stringify({ id: "probe-1", op: "probe" });
  const second = JSON.stringify({ id: "probe-2", op: "probe" });
  assert.deepEqual(await collectLines([`${first}\n${second}\n`], 128), [first, second]);
});

test("turn-host stdin continues to ignore empty lines", async () => {
  const probe = JSON.stringify({ id: "probe-1", op: "probe" });
  assert.deepEqual(await collectLines([`\n\r\n${probe}\n\n`], 128), [probe]);
});

test("turn-host stdin rejects newline-terminated overflow before parsing", async () => {
  const max = 32;
  await assert.rejects(
    collectLines([`${"x".repeat(max + 1)}\n`], max),
    (error) => error instanceof AnalystTurnHostInputError && /record exceeded/.test(error.message),
  );
});

test("turn-host stdin rejects no-newline overflow promptly without waiting for EOF", async () => {
  const max = 32;
  const input = new PassThrough();
  const consume = (async () => {
    for await (const _line of readAnalystTurnLines(input, max)) {
    }
  })();
  input.write(Buffer.alloc(max + 2, 0x78));
  try {
    await Promise.race([
      assert.rejects(consume, /record exceeded/),
      new Promise((_, reject) => setTimeout(() => reject(new Error("framing overflow did not fail promptly")), 250)),
    ]);
  } finally {
    input.destroy();
  }
});

test("turn-host stdin rejects an oversized EOF tail", async () => {
  await assert.rejects(collectLines([Buffer.alloc(34, 0x78)], 32), /record exceeded/);
});

test("turn-host stdin yields a complete first request before a later oversized request in the same chunk", async () => {
  const max = 48;
  const first = JSON.stringify({ id: "probe-1", op: "probe" });
  const iterator = readAnalystTurnLines(
    Readable.from([`${first}\n${"x".repeat(max + 1)}\n`]),
    max,
  )[Symbol.asyncIterator]();
  assert.deepEqual(await iterator.next(), { value: first, done: false });
  await assert.rejects(iterator.next(), /record exceeded/);
});

test("turn-host framing input failure shuts down the restricted Pi child", async () => {
  const temp = await mkdtemp(join(tmpdir(), "world-machine-m256-framing-"));
  const fakePi = join(temp, "fake-pi.mjs");
  const readyFile = join(temp, "ready.txt");
  await writeFile(
    fakePi,
    [
      "#!/usr/bin/env node",
      'import { writeFileSync } from "node:fs";',
      "writeFileSync(process.env.FAKE_PI_READY_FILE, String(process.pid));",
      "setInterval(() => {}, 1000);",
      "",
    ].join("\n"),
  );
  await chmod(fakePi, 0o755);

  const stdin = {
    async *[Symbol.asyncIterator]() {
      await waitForFile(readyFile);
      throw new AnalystTurnHostInputError("analyst turn input record exceeded the transport limit");
    },
    destroy() {},
  };
  const stdout = new PassThrough();
  const signalSource = new EventEmitter();
  const run = runAnalystTurnHost({
    stdin,
    stdout,
    argv: ["left.world", "right.world", "--provider", "fake", "--model", "fake"],
    env: {
      ...process.env,
      PI_PROGRAM: fakePi,
      WORLD_MACHINE_ANALYST_PROGRAM: process.execPath,
      FAKE_PI_READY_FILE: readyFile,
    },
    signalSource,
  });

  try {
    await assert.rejects(run, /record exceeded/);
    const piPid = Number(await readFile(readyFile, "utf8"));
    assert.ok(Number.isInteger(piPid) && piPid > 0);
    assert.equal(
      processExists(piPid),
      false,
      "restricted Pi child must not survive framing input failure",
    );
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
});

test("turn-host framing reader limit cannot exceed the production ceiling", async () => {
  await assert.rejects(
    collectLines([], 64 * 1024 * 1024 + 1),
    (error) => error instanceof AnalystTurnHostInputError && /1\.\.=67108864/.test(error.message),
  );
});

async function waitForFile(path) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (existsSync(path)) return;
    await sleep(20);
  }
  throw new Error(`timed out waiting for ${path}`);
}

function processExists(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error?.code === "ESRCH") return false;
    throw error;
  }
}
