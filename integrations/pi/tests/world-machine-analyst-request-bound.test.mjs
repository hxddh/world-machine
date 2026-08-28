import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";
import {
  AnalystBridgeError,
  AnalystJsonlClient,
} from "../world-machine-analyst-client.mjs";

const PRODUCTION_MAX_REQUEST_BYTES = 64 * 1024 * 1024;

function catalogRecord(toolName = "world.first-divergence") {
  return {
    protocol: "world-machine-readonly-tools",
    version: 1,
    type: "catalog",
    tools: toolName === null ? [] : [{ name: toolName, read_only: true }],
  };
}

function resultRecord(request) {
  return {
    protocol: "world-machine-readonly-tools",
    version: 1,
    type: "result",
    call_id: request.call_id,
    tool: request.tool,
    output: { echoed: request.input },
  };
}

function controlledClient({ maxRequestBytes, onRequest } = {}) {
  const child = new EventEmitter();
  child.stdin = new PassThrough();
  child.stdout = new PassThrough();
  child.exitCode = null;
  child.signalCode = null;
  const killSignals = [];
  const writes = [];
  const requests = [];

  child.kill = (signal = "SIGTERM") => {
    if (child.exitCode !== null || child.signalCode !== null) return false;
    killSignals.push(signal);
    child.signalCode = signal;
    queueMicrotask(() => child.emit("exit", null, signal));
    return true;
  };
  child.stdin.on("finish", () => child.kill("SIGTERM"));

  const originalWrite = child.stdin.write.bind(child.stdin);
  child.stdin.write = (chunk, encoding, callback) => {
    writes.push(Buffer.from(chunk));
    return originalWrite(chunk, encoding, callback);
  };

  let buffer = "";
  child.stdin.setEncoding("utf8");
  child.stdin.on("data", (chunk) => {
    buffer += chunk;
    while (true) {
      const newline = buffer.indexOf("\n");
      if (newline < 0) return;
      const line = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      if (line.length === 0) continue;
      const request = JSON.parse(line);
      requests.push(request);
      if (onRequest) {
        onRequest(request, child.stdout, requests.length);
      } else if (request.op === "list-tools") {
        child.stdout.write(`${JSON.stringify(catalogRecord())}\n`);
      } else {
        child.stdout.write(`${JSON.stringify(resultRecord(request))}\n`);
      }
    }
  });

  const options = maxRequestBytes === undefined ? {} : { maxRequestBytes };
  return {
    client: new AnalystJsonlClient(child, options),
    child,
    killSignals,
    writes,
    requests,
  };
}

function serializedInvoke(callId, tool, input) {
  return JSON.stringify({
    op: "invoke",
    call_id: callId,
    tool,
    input,
  });
}

function isRequestOverflow(error) {
  return (
    error instanceof AnalystBridgeError &&
    error.details.kind === "request-overflow" &&
    /request payload exceeded/.test(error.message)
  );
}

test("M260 request limit is lower-only and cannot exceed the production ceiling", () => {
  const child = new EventEmitter();
  child.stdin = new PassThrough();
  child.stdout = new PassThrough();
  child.exitCode = null;
  child.signalCode = null;
  child.kill = () => true;

  assert.throws(
    () => new AnalystJsonlClient(child, { maxRequestBytes: PRODUCTION_MAX_REQUEST_BYTES + 1 }),
    (error) =>
      error instanceof AnalystBridgeError &&
      /max request bytes/.test(error.message) &&
      /production ceiling/.test(error.message),
  );
});

test("M260 listTools uses the same request bound and writes nothing on local overflow", async () => {
  const payloadBytes = Buffer.byteLength(JSON.stringify({ op: "list-tools" }), "utf8");
  const { client, child, writes, requests, killSignals } = controlledClient({
    maxRequestBytes: payloadBytes - 1,
  });
  try {
    await assert.rejects(client.listTools(), isRequestOverflow);
    assert.equal(writes.length, 0);
    assert.equal(requests.length, 0);
    assert.equal(child.signalCode, null);
    assert.deepEqual(killSignals, []);
  } finally {
    await client.shutdown();
  }
});

test("M260 exact-limit invoke writes the serialized payload once plus one framing LF", async () => {
  const callId = "exact-1";
  const tool = "world.first-divergence";
  const input = { root: "event-9", padding: "abc" };
  const serialized = serializedInvoke(callId, tool, input);
  const payloadBytes = Buffer.byteLength(serialized, "utf8");
  const { client, writes, requests } = controlledClient({ maxRequestBytes: payloadBytes });
  try {
    assert.deepEqual(await client.invoke(callId, tool, input), { echoed: input });
    assert.equal(requests.length, 1);
    assert.equal(writes.length, 1);
    assert.equal(writes[0].length, payloadBytes + 1);
    assert.deepEqual(writes[0].subarray(0, -1), Buffer.from(serialized, "utf8"));
    assert.equal(writes[0].at(-1), 0x0a);
  } finally {
    await client.shutdown();
  }
});

test("M260 limit-plus-one invoke is local, nonfatal, and the same child remains reusable", async () => {
  const callId = "oversized-1";
  const tool = "world.first-divergence";
  const input = { padding: "x".repeat(128) };
  const serialized = serializedInvoke(callId, tool, input);
  const { client, child, writes, requests, killSignals } = controlledClient({
    maxRequestBytes: Buffer.byteLength(serialized, "utf8") - 1,
  });
  try {
    await assert.rejects(client.invoke(callId, tool, input), isRequestOverflow);
    assert.equal(writes.length, 0);
    assert.equal(requests.length, 0);
    assert.equal(child.signalCode, null);
    assert.deepEqual(killSignals, []);

    const tools = await client.listTools();
    assert.equal(tools[0].name, "world.first-divergence");
    assert.equal(requests.length, 1);
  } finally {
    await client.shutdown();
  }
});

test("M260 counts UTF-8 bytes rather than JavaScript string length", async () => {
  const callId = "utf8-1";
  const tool = "world.first-divergence";
  const input = { note: "é" };
  const serialized = serializedInvoke(callId, tool, input);
  assert.ok(Buffer.byteLength(serialized, "utf8") > serialized.length);
  const { client, writes, requests } = controlledClient({
    maxRequestBytes: serialized.length,
  });
  try {
    await assert.rejects(client.invoke(callId, tool, input), isRequestOverflow);
    assert.equal(writes.length, 0);
    assert.equal(requests.length, 0);
  } finally {
    await client.shutdown();
  }
});

test("M260 local stringify failure leaves no stale waiter and the next request succeeds", async () => {
  const { client, child, writes, requests } = controlledClient();
  try {
    await assert.rejects(
      client.invoke("bigint-1", "world.first-divergence", { value: 1n }),
      (error) =>
        error instanceof AnalystBridgeError &&
        error.details.kind === "request-serialization",
    );
    assert.equal(writes.length, 0);
    assert.equal(requests.length, 0);
    assert.equal(child.signalCode, null);

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 250);
    try {
      const tools = await client.listTools(controller.signal);
      assert.equal(tools[0].name, "world.first-divergence");
    } finally {
      clearTimeout(timer);
    }
    assert.equal(requests.length, 1);
  } finally {
    await client.shutdown();
  }
});

test("M260 local overflow does not consume a response already queued for the next valid call", async () => {
  let first = true;
  const callId = "queued-overflow";
  const tool = "world.first-divergence";
  const input = { padding: "x".repeat(128) };
  const serialized = serializedInvoke(callId, tool, input);
  const { client, requests, writes } = controlledClient({
    maxRequestBytes: Buffer.byteLength(serialized, "utf8") - 1,
    onRequest(request, stdout) {
      if (first) {
        first = false;
        assert.equal(request.op, "list-tools");
        stdout.write(
          `${JSON.stringify(catalogRecord("world.first"))}\n${JSON.stringify(catalogRecord("world.queued"))}\n`,
        );
        return;
      }
      if (request.op === "list-tools") {
        stdout.write(`${JSON.stringify(catalogRecord("world.third"))}\n`);
        return;
      }
      stdout.write(`${JSON.stringify(resultRecord(request))}\n`);
    },
  });

  try {
    const firstTools = await client.listTools();
    assert.equal(firstTools[0].name, "world.first");
    const writesBeforeOverflow = writes.length;
    const requestsBeforeOverflow = requests.length;

    await assert.rejects(client.invoke(callId, tool, input), isRequestOverflow);
    assert.equal(writes.length, writesBeforeOverflow);
    assert.equal(requests.length, requestsBeforeOverflow);

    const queued = await client.listTools();
    assert.equal(queued[0].name, "world.queued");
  } finally {
    await client.shutdown();
  }
});

test("M260 accepted invoke serializes its input exactly once", async () => {
  let serializations = 0;
  const input = {
    toJSON() {
      serializations += 1;
      return { root: "event-9" };
    },
  };
  const { client, requests } = controlledClient();
  try {
    assert.deepEqual(
      await client.invoke("once-1", "world.first-divergence", input),
      { echoed: { root: "event-9" } },
    );
    assert.equal(serializations, 1);
    assert.equal(requests.length, 1);
  } finally {
    await client.shutdown();
  }
});
