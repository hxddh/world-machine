import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, writeFile, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import {
  ANALYST_TURN_PROTOCOL,
  ANALYST_TURN_PROTOCOL_VERSION,
  AnalystTurnHost,
  AnalystTurnHostInputError,
} from "../world-machine-analyst-turn-host.mjs";
import {
  PiAnalystRpcCommandError,
  PiAnalystRpcProtocolError,
} from "../world-machine-analyst-rpc.mjs";

class ScriptedSession {
  constructor(script) {
    this.script = [...script];
    this.prompts = [];
    this.probes = [];
    this.shutdownCount = 0;
  }

  async probe(options) {
    this.probes.push(options);
    return { requestId: "probe" };
  }

  async prompt(prompt, options) {
    this.prompts.push({ prompt, options });
    const next = this.script.shift();
    if (next instanceof Error) throw next;
    return next;
  }

  async shutdown() {
    this.shutdownCount += 1;
  }
}

test("host maps a completed analyst turn into provider-neutral protocol v1", async () => {
  const session = new ScriptedSession([{
    requestId: "world-analyst-1",
    text: "The divergence starts at event 7.",
    toolCalls: [{
      toolCallId: "tool-1",
      toolName: "world_first_divergence",
      args: { root: "event-7" },
      result: {
        content: [{ type: "text", text: "opaque provider content" }],
        details: {
          worldMachineTool: "world.first-divergence",
          output: { divergence_depth: 1 },
        },
      },
      isError: false,
    }],
    extensionErrors: [{
      extensionPath: "world-machine-analyst.mjs",
      event: "tool",
      error: "recoverable extension diagnostic",
    }],
    events: ["agent_start", "tool_execution_start", "tool_execution_end", "message_end", "agent_settled"],
  }]);
  const host = new AnalystTurnHost(session);

  const response = await host.handle({ id: "ask-1", op: "ask", prompt: "What changed?", timeout_ms: 5000 });

  assert.equal(response.protocol, ANALYST_TURN_PROTOCOL);
  assert.equal(response.version, ANALYST_TURN_PROTOCOL_VERSION);
  assert.equal(response.type, "result");
  assert.equal(response.id, "ask-1");
  assert.deepEqual(response.turn, {
    request_id: "world-analyst-1",
    text: "The divergence starts at event 7.",
    tool_calls: [{
      call_id: "tool-1",
      tool: "world.first-divergence",
      input: { root: "event-7" },
      output: { divergence_depth: 1 },
      is_error: false,
    }],
    runtime_errors: [{
      kind: "extension",
      source: "world-machine-analyst.mjs",
      message: "recoverable extension diagnostic",
    }],
  });
  assert.equal("events" in response.turn, false);
  assert.equal(JSON.stringify(response).includes("world_first_divergence"), false);
  assert.deepEqual(session.prompts, [{ prompt: "What changed?", options: { timeoutMs: 5000 } }]);
});

test("startup probe is provider-neutral and does not dispatch a model prompt", async () => {
  const session = new ScriptedSession([]);
  const host = new AnalystTurnHost(session);
  const response = await host.handle({ id: "probe-1", op: "probe", timeout_ms: 2500 });
  assert.deepEqual(response, {
    protocol: ANALYST_TURN_PROTOCOL,
    version: ANALYST_TURN_PROTOCOL_VERSION,
    type: "ready",
    id: "probe-1",
  });
  assert.deepEqual(session.probes, [{ timeoutMs: 2500 }]);
  assert.deepEqual(session.prompts, []);
});

test("command errors are correlated and non-fatal while protocol errors are fatal", async () => {
  const session = new ScriptedSession([
    new PiAnalystRpcCommandError("rejected", { reason: "busy", provider_response: "opaque" }),
    new PiAnalystRpcProtocolError("bad event", { type: "mismatch", event: "opaque" }),
  ]);
  const host = new AnalystTurnHost(session);

  const rejected = await host.handle({ id: "ask-1", op: "ask", prompt: "one" });
  assert.deepEqual(rejected, {
    protocol: ANALYST_TURN_PROTOCOL,
    version: ANALYST_TURN_PROTOCOL_VERSION,
    type: "error",
    id: "ask-1",
    error: { kind: "command", fatal: false, message: "rejected" },
  });

  const broken = await host.handle({ id: "ask-2", op: "ask", prompt: "two" });
  assert.deepEqual(broken, {
    protocol: ANALYST_TURN_PROTOCOL,
    version: ANALYST_TURN_PROTOCOL_VERSION,
    type: "error",
    id: "ask-2",
    error: { kind: "protocol", fatal: true, message: "bad event" },
  });
  assert.equal(JSON.stringify(broken).includes("opaque"), false);
});

test("request shape is strict and archive paths cannot appear per turn", async () => {
  const host = new AnalystTurnHost(new ScriptedSession([]));
  await assert.rejects(
    host.handle({ id: "ask-1", op: "ask", prompt: "x", left_archive: "/tmp/other.world" }),
    (error) => error instanceof AnalystTurnHostInputError && /unknown/.test(error.message),
  );
  await assert.rejects(
    host.handle({ id: "ask-1", op: "ask", prompt: "x", timeout_ms: 0 }),
    (error) => error instanceof AnalystTurnHostInputError && /positive integer/.test(error.message),
  );
});

test("real turn-host process reuses one restricted Pi child for two asks", async () => {
  const temp = await mkdtemp(join(tmpdir(), "world-machine-m220-"));
  const fakePi = join(temp, "fake-pi.mjs");
  await writeFile(fakePi, `#!/usr/bin/env node
let buffer = Buffer.alloc(0);
let turn = 0;
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const newline = buffer.indexOf(0x0a);
    if (newline < 0) break;
    const line = buffer.subarray(0, newline).toString("utf8");
    buffer = buffer.subarray(newline + 1);
    if (!line) continue;
    const request = JSON.parse(line);
    if (request.type === "get_state") {
      emit({
        type: "response",
        id: request.id,
        command: "get_state",
        success: true,
        data: { model: null, thinkingLevel: "off", isStreaming: false, isCompacting: false }
      });
      continue;
    }
    if (request.type === "get_commands") {
      emit({
        type: "response",
        id: request.id,
        command: "get_commands",
        success: true,
        data: { commands: [{ name: "world-machine-analyst-ready", source: "extension" }] }
      });
      continue;
    }
    turn += 1;
    emit({ type: "response", id: request.id, command: "prompt", success: true });
    emit({ type: "tool_execution_start", toolCallId: "tool-" + turn, toolName: "world_first_divergence", args: { root: "event-" + turn } });
    emit({
      type: "tool_execution_end",
      toolCallId: "tool-" + turn,
      toolName: "world_first_divergence",
      result: {
        content: [{ type: "text", text: "provider-only" }],
        details: {
          worldMachineTool: "world.first-divergence",
          output: { turn }
        }
      },
      isError: false
    });
    emit({ type: "message_end", message: { role: "assistant", content: "answer-" + turn } });
    emit({ type: "agent_settled" });
  }
});
function emit(value) { process.stdout.write(JSON.stringify(value) + "\\n"); }
`);
  await chmod(fakePi, 0o755);

  const repoRoot = resolve(new URL("../../..", import.meta.url).pathname);
  const hostPath = resolve(repoRoot, "integrations/pi/world-machine-analyst-turn-host.mjs");
  const child = spawn(process.execPath, [hostPath, "left.world", "right.world", "--provider", "fake", "--model", "fake"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      PI_PROGRAM: fakePi,
      WORLD_MACHINE_ANALYST_PROGRAM: process.execPath,
    },
    stdio: ["pipe", "pipe", "pipe"],
  });
  const exit = new Promise((resolveCode) => child.on("exit", (exitCode) => resolveCode(exitCode)));

  child.stdin.write(`${JSON.stringify({ id: "probe-1", op: "probe", timeout_ms: 2000 })}\n`);
  child.stdin.write(`${JSON.stringify({ id: "ask-1", op: "ask", prompt: "first" })}\n`);
  child.stdin.write(`${JSON.stringify({ id: "ask-2", op: "ask", prompt: "second" })}\n`);
  child.stdin.end();

  const [stdout, stderr, code] = await Promise.all([
    collect(child.stdout),
    collect(child.stderr),
    exit,
  ]);
  await rm(temp, { recursive: true, force: true });

  assert.equal(code, 0, stderr);
  const responses = stdout.trim().split("\n").map((line) => JSON.parse(line));
  assert.equal(responses.length, 3);
  assert.equal(responses[0].type, "ready");
  assert.equal(responses[0].id, "probe-1");
  assert.equal(responses[1].id, "ask-1");
  assert.equal(responses[1].turn.text, "answer-1");
  assert.deepEqual(responses[1].turn.tool_calls[0], {
    call_id: "tool-1",
    tool: "world.first-divergence",
    input: { root: "event-1" },
    output: { turn: 1 },
    is_error: false,
  });
  assert.equal(responses[2].id, "ask-2");
  assert.equal(responses[2].turn.text, "answer-2");
  assert.equal(responses[2].turn.tool_calls[0].call_id, "tool-2");
  assert.equal(responses[2].turn.tool_calls[0].tool, "world.first-divergence");
  assert.equal(JSON.stringify(responses).includes("provider-only"), false);
  assert.equal(JSON.stringify(responses).includes("world_first_divergence"), false);
});

async function collect(stream) {
  let output = "";
  stream.setEncoding("utf8");
  for await (const chunk of stream) output += chunk;
  return output;
}
