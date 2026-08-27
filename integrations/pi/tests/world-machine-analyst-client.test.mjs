import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import test from "node:test";
import worldMachineAnalyst from "../world-machine-analyst.mjs";
import {
  AnalystBridgeError,
  AnalystJsonlClient,
  providerSafeToolName,
} from "../world-machine-analyst-client.mjs";

function fakeServer(script) {
  return AnalystJsonlClient.spawn(process.execPath, ["-e", script], { stderr: "pipe" });
}

function controlledClient(onRequest = () => {}, options = {}, control = {}) {
  const child = new EventEmitter();
  child.stdin = new PassThrough();
  child.stdout = new PassThrough();
  child.exitCode = null;
  child.signalCode = null;
  const killSignals = [];
  child.kill = (signal = "SIGTERM") => {
    if (child.exitCode !== null || child.signalCode !== null) return false;
    killSignals.push(signal);
    if (signal === "SIGTERM" && control.ignoreSigterm === true) return true;
    child.signalCode = signal;
    queueMicrotask(() => child.emit("exit", null, signal));
    return true;
  };
  child.stdin.on("finish", () => child.kill("SIGTERM"));

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
      onRequest(JSON.parse(line), child.stdout);
    }
  });

  return { client: new AnalystJsonlClient(child, options), child, killSignals };
}

function catalogRecord(toolName = null, extra = {}) {
  return JSON.stringify({
    protocol: "world-machine-readonly-tools",
    version: 1,
    type: "catalog",
    tools: toolName === null ? [] : [{ name: toolName }],
    ...extra,
  });
}

function exactSizedCatalogRecord(byteLength) {
  const empty = catalogRecord(null, { padding: "" });
  const fillerBytes = byteLength - Buffer.byteLength(empty);
  assert.ok(fillerBytes >= 0, "test limit must fit the catalog envelope");
  const record = catalogRecord(null, { padding: "x".repeat(fillerBytes) });
  assert.equal(Buffer.byteLength(record), byteLength);
  return record;
}

const SERVER = String.raw`
let buffer = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const newline = buffer.indexOf(0x0a);
    if (newline < 0) break;
    const line = buffer.subarray(0, newline).toString("utf8");
    buffer = buffer.subarray(newline + 1);
    if (!line) continue;
    const request = JSON.parse(line);
    if (request.op === "list-tools") {
      emit({
        protocol: "world-machine-readonly-tools",
        version: 1,
        type: "catalog",
        tools: [{
          name: "world.first-divergence",
          description: "Find the first causal divergence.",
          read_only: true,
          input_schema: { type: "object", properties: { root: { type: "string" } }, required: ["root"] }
        }]
      });
    } else if (request.tool === "world.missing") {
      emit({
        protocol: "world-machine-readonly-tools",
        version: 1,
        type: "error",
        call_id: request.call_id,
        tool: request.tool,
        error: { kind: "unknown-tool", message: "missing" }
      });
    } else {
      emit({
        protocol: "world-machine-readonly-tools",
        version: 1,
        type: "result",
        call_id: request.call_id,
        tool: request.tool,
        output: { echoed: request.input }
      });
    }
  }
});
function emit(value) { process.stdout.write(JSON.stringify(value) + "\n"); }
`;

test("catalog and invoke share one single-flight child session", async () => {
  const client = fakeServer(SERVER);
  try {
    const tools = await client.listTools();
    assert.equal(tools.length, 1);
    assert.equal(tools[0].name, "world.first-divergence");
    assert.equal(tools[0].read_only, true);

    const output = await client.invoke("call-1", "world.first-divergence", { root: "event-9" });
    assert.deepEqual(output, { echoed: { root: "event-9" } });
  } finally {
    await client.shutdown();
  }
});

test("actual Pi extension registers the host catalog and executes through it", async () => {
  const previous = {
    program: process.env.WORLD_MACHINE_ANALYST_PROGRAM,
    left: process.env.WORLD_MACHINE_LEFT_ARCHIVE,
    right: process.env.WORLD_MACHINE_RIGHT_ARCHIVE,
  };
  process.env.WORLD_MACHINE_ANALYST_PROGRAM = process.execPath;
  process.env.WORLD_MACHINE_LEFT_ARCHIVE = "-e";
  process.env.WORLD_MACHINE_RIGHT_ARCHIVE = SERVER;

  const handlers = new Map();
  const registered = [];
  const registeredCommands = [];
  const activeHistory = [];
  const pi = {
    on(event, handler) {
      handlers.set(event, handler);
    },
    setActiveTools(names) {
      activeHistory.push([...names]);
    },
    registerTool(tool) {
      registered.push(tool);
    },
    registerCommand(name, command) {
      registeredCommands.push({ name, command });
    },
  };

  worldMachineAnalyst(pi);
  try {
    await handlers.get("session_start")();
    assert.deepEqual(activeHistory[0], []);
    assert.deepEqual(activeHistory.at(-1), ["world_first_divergence"]);
    assert.equal(registered.length, 1);
    assert.equal(registeredCommands.length, 1);
    assert.equal(registeredCommands[0].name, "world-machine-analyst-ready");

    const tool = registered[0];
    assert.equal(tool.name, "world_first_divergence");
    assert.equal(tool.label, "world.first-divergence");
    assert.equal(tool.executionMode, "sequential");
    assert.deepEqual(tool.parameters, {
      type: "object",
      properties: { root: { type: "string" } },
      required: ["root"],
    });

    const result = await tool.execute(
      "pi-call-1",
      { root: "event-9" },
      new AbortController().signal,
    );
    assert.equal(result.details.worldMachineTool, "world.first-divergence");
    assert.deepEqual(result.details.output, { echoed: { root: "event-9" } });
    assert.deepEqual(JSON.parse(result.content[0].text), { echoed: { root: "event-9" } });
  } finally {
    if (handlers.has("session_shutdown")) {
      await handlers.get("session_shutdown")();
    }
    restoreEnv("WORLD_MACHINE_ANALYST_PROGRAM", previous.program);
    restoreEnv("WORLD_MACHINE_LEFT_ARCHIVE", previous.left);
    restoreEnv("WORLD_MACHINE_RIGHT_ARCHIVE", previous.right);
  }
  assert.deepEqual(activeHistory.at(-1), []);
});

test("remote tool errors stay correlated and do not poison the session", async () => {
  const client = fakeServer(SERVER);
  try {
    await assert.rejects(
      client.invoke("missing-1", "world.missing", {}),
      (error) =>
        error instanceof AnalystBridgeError &&
        error.details.callId === "missing-1" &&
        error.details.tool === "world.missing" &&
        error.details.remoteError.kind === "unknown-tool",
    );
    const tools = await client.listTools();
    assert.equal(tools[0].name, "world.first-divergence");
  } finally {
    await client.shutdown();
  }
});

test("protocol and call correlation are strict", async () => {
  const badProtocol = fakeServer(String.raw`
    process.stdin.once("data", () => process.stdout.write(JSON.stringify({ protocol: "other", version: 1, type: "catalog", tools: [] }) + "\n"));
  `);
  try {
    await assert.rejects(badProtocol.listTools(), /unexpected analyst protocol/);
  } finally {
    await badProtocol.shutdown();
  }

  const badCorrelation = fakeServer(String.raw`
    process.stdin.once("data", () => process.stdout.write(JSON.stringify({
      protocol: "world-machine-readonly-tools", version: 1, type: "result",
      call_id: "other-call", tool: "world.other", output: {}
    }) + "\n"));
  `);
  try {
    await assert.rejects(
      badCorrelation.invoke("call-1", "world.first-divergence", {}),
      /correlation mismatch/,
    );
  } finally {
    await badCorrelation.shutdown();
  }
});

test("abort terminates the bound analyst child", async () => {
  const client = fakeServer(String.raw`process.stdin.resume();`);
  const controller = new AbortController();
  const pending = client.listTools(controller.signal);
  controller.abort();
  await assert.rejects(pending, /aborted/);
  await client.shutdown();
});

test("Analyst JSONL framing accepts an exact-limit LF record", async () => {
  const maxRecordBytes = 192;
  const record = exactSizedCatalogRecord(maxRecordBytes);
  const { client } = controlledClient((request, stdout) => {
    assert.equal(request.op, "list-tools");
    stdout.write(`${record}\n`);
  }, { maxRecordBytes });

  try {
    assert.deepEqual(await client.listTools(), []);
  } finally {
    await client.shutdown();
  }
});

test("Analyst JSONL framing accepts an exact-limit CRLF record", async () => {
  const maxRecordBytes = 192;
  const record = exactSizedCatalogRecord(maxRecordBytes);
  const { client } = controlledClient((request, stdout) => {
    assert.equal(request.op, "list-tools");
    stdout.write(`${record}\r\n`);
  }, { maxRecordBytes });

  try {
    assert.deepEqual(await client.listTools(), []);
  } finally {
    await client.shutdown();
  }
});

test("Analyst JSONL framing accepts a response split across one-byte chunks", async () => {
  const maxRecordBytes = 256;
  const record = `${catalogRecord("world.split")}\n`;
  const { client } = controlledClient((request, stdout) => {
    assert.equal(request.op, "list-tools");
    for (const byte of Buffer.from(record)) stdout.write(Buffer.from([byte]));
  }, { maxRecordBytes });

  try {
    const tools = await client.listTools();
    assert.equal(tools[0].name, "world.split");
  } finally {
    await client.shutdown();
  }
});

test("Analyst JSONL framing preserves multiple records from one stdout chunk", async () => {
  let requests = 0;
  const { client } = controlledClient((request, stdout) => {
    requests += 1;
    if (requests === 1) {
      assert.equal(request.op, "list-tools");
      stdout.write(`${catalogRecord("world.first")}\n${catalogRecord("world.second")}\n`);
    }
  }, { maxRecordBytes: 256 });

  try {
    const first = await client.listTools();
    const second = await client.listTools();
    assert.equal(first[0].name, "world.first");
    assert.equal(second[0].name, "world.second");
    assert.equal(requests, 2);
  } finally {
    await client.shutdown();
  }
});

test("Analyst JSONL framing rejects newline-terminated overflow before JSON parsing", async () => {
  const maxRecordBytes = 64;
  const { client, child } = controlledClient((_request, stdout) => {
    stdout.write(`${"x".repeat(maxRecordBytes + 1)}\n`);
  }, { maxRecordBytes });

  await assert.rejects(
    client.listTools(),
    (error) =>
      error instanceof AnalystBridgeError &&
      /record exceeded/.test(error.message) &&
      !/invalid analyst response JSON/.test(error.message),
  );
  assert.equal(child.signalCode, "SIGTERM");
});

test("Analyst JSONL framing rejects a no-newline overflow promptly", async () => {
  const maxRecordBytes = 64;
  const { client, child } = controlledClient((_request, stdout) => {
    stdout.write(Buffer.alloc(maxRecordBytes + 2, 0x78));
  }, { maxRecordBytes });

  const started = Date.now();
  await assert.rejects(
    client.listTools(),
    (error) => error instanceof AnalystBridgeError && /record exceeded/.test(error.message),
  );
  assert.ok(Date.now() - started < 500, "framing overflow should fail without waiting for EOF");
  assert.equal(child.signalCode, "SIGTERM");
});

test("Analyst JSONL framing poisons the client and prevents later reuse", async () => {
  const maxRecordBytes = 64;
  let requests = 0;
  const { client } = controlledClient((_request, stdout) => {
    requests += 1;
    stdout.write(`${"x".repeat(maxRecordBytes + 1)}\n`);
  }, { maxRecordBytes });

  await assert.rejects(client.listTools(), /record exceeded/);
  await assert.rejects(client.listTools(), /record exceeded/);
  assert.equal(requests, 1);
});

test("Analyst JSONL framing ignores valid-looking bytes after an oversized prefix", async () => {
  const maxRecordBytes = 64;
  const { client } = controlledClient((_request, stdout) => {
    stdout.write(
      `${"x".repeat(maxRecordBytes + 1)}\n${catalogRecord("world.must-not-recover")}\n`,
    );
  }, { maxRecordBytes });

  await assert.rejects(client.listTools(), /record exceeded/);
  await assert.rejects(client.listTools(), /record exceeded/);
});

test("Analyst JSONL framing makes same-chunk contamination win the active request", async () => {
  const maxRecordBytes = 128;
  const { client, child } = controlledClient((_request, stdout) => {
    stdout.write(
      `${catalogRecord("world.apparently-valid")}\n${"x".repeat(maxRecordBytes + 1)}\n`,
    );
  }, { maxRecordBytes });

  await assert.rejects(
    client.listTools(),
    (error) => error instanceof AnalystBridgeError && /record exceeded/.test(error.message),
  );
  assert.equal(child.signalCode, "SIGTERM");
});

test("Analyst JSONL framing escalates idle overflow cleanup when SIGTERM is ignored", async () => {
  const maxRecordBytes = 32;
  const { client, child, killSignals } = controlledClient(
    () => {},
    { maxRecordBytes },
    { ignoreSigterm: true },
  );

  child.stdout.write(Buffer.alloc(maxRecordBytes + 2, 0x78));
  await assert.rejects(client.listTools(), /record exceeded/);
  await new Promise((resolve) => setTimeout(resolve, 700));
  assert.deepEqual(killSignals, ["SIGTERM", "SIGKILL"]);
  assert.equal(child.signalCode, "SIGKILL");
});

test("Analyst JSONL framing observes the pending response when request writing fails", async () => {
  const maxRecordBytes = 32;
  const { client, child } = controlledClient(() => {}, { maxRecordBytes });
  const originalWrite = child.stdin.write.bind(child.stdin);
  child.stdin.write = (chunk, encoding, callback) => {
    child.stdout.write(Buffer.alloc(maxRecordBytes + 2, 0x78));
    queueMicrotask(() => callback(new Error("simulated EPIPE")));
    return true;
  };

  try {
    await assert.rejects(
      client.listTools(),
      (error) => error instanceof AnalystBridgeError && /record exceeded/.test(error.message),
    );
    await new Promise((resolve) => setImmediate(resolve));
  } finally {
    child.stdin.write = originalWrite;
  }
});

test("Analyst JSONL framing still escalates when the exit wait rejects on child error", async () => {
  const maxRecordBytes = 32;
  const { client, child, killSignals } = controlledClient(
    () => {},
    { maxRecordBytes },
    { ignoreSigterm: true },
  );

  child.stdout.write(Buffer.alloc(maxRecordBytes + 2, 0x78));
  child.emit("error", new Error("simulated child error during termination"));
  await assert.rejects(client.listTools(), /record exceeded/);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(killSignals, ["SIGTERM", "SIGKILL"]);
  assert.equal(child.signalCode, "SIGKILL");
});

test("provider tool names are deterministic and constrained", () => {
  assert.equal(providerSafeToolName("world.first-divergence"), "world_first_divergence");
  assert.equal(providerSafeToolName("7.bad tool"), "world_7_bad_tool");
  assert.ok(providerSafeToolName("x".repeat(100)).length <= 64);
});

function restoreEnv(name, value) {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}
