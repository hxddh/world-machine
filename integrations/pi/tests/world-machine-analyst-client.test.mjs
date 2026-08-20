import assert from "node:assert/strict";
import test from "node:test";
import worldMachineAnalyst from "../world-machine-analyst.mjs";
import { AnalystBridgeError, AnalystJsonlClient, providerSafeToolName } from "../world-machine-analyst-client.mjs";

function fakeServer(script) {
  return AnalystJsonlClient.spawn(process.execPath, ["-e", script], { stderr: "pipe" });
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

test("provider tool names are deterministic and constrained", () => {
  assert.equal(providerSafeToolName("world.first-divergence"), "world_first_divergence");
  assert.equal(providerSafeToolName("7.bad tool"), "world_7_bad_tool");
  assert.ok(providerSafeToolName("x".repeat(100)).length <= 64);
});

function restoreEnv(name, value) {
  if (value === undefined) delete process.env[name];
  else process.env[name] = value;
}
