import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import test from "node:test";
import {
  PiAnalystRpcCommandError,
  PiAnalystRpcProtocolError,
  PiAnalystRpcSession,
  PiAnalystRpcTransportError,
} from "../world-machine-analyst-rpc.mjs";

function fakeSession(script) {
  const child = spawn(process.execPath, ["-e", script], {
    stdio: ["pipe", "pipe", "pipe"],
  });
  return new PiAnalystRpcSession(child);
}

const REUSABLE_SERVER = String.raw`
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
    turn += 1;
    emit({ type: "response", id: request.id, command: "prompt", success: true });
    emit({ type: "agent_start" });
    emit({
      type: "tool_execution_start",
      toolCallId: "tool-" + turn,
      toolName: "world_first_divergence",
      args: { root: "event-" + turn }
    });
    emit({
      type: "tool_execution_end",
      toolCallId: "tool-" + turn,
      toolName: "world_first_divergence",
      result: { content: [{ type: "text", text: "evidence-" + turn }] },
      isError: turn === 2
    });
    emit({
      type: "message_end",
      message: { role: "assistant", content: [{ type: "text", text: "draft-" + turn }] }
    });
    emit({ type: "agent_end", messages: [] });
    emit({ type: "agent_start" });
    emit({
      type: "message_end",
      message: { role: "assistant", content: [{ type: "text", text: "final-" + turn }] }
    });
    emit({ type: "agent_end", messages: [] });
    emit({ type: "agent_settled" });
  }
});
function emit(value) { process.stdout.write(JSON.stringify(value) + "\n"); }
`;

test("two prompts reuse one Pi process and settle after low-level agent_end", async () => {
  const session = fakeSession(REUSABLE_SERVER);
  const processId = session.id();
  try {
    const first = await session.prompt("first question", { timeoutMs: 2000 });
    assert.equal(session.id(), processId);
    assert.equal(first.requestId, "world-analyst-1");
    assert.equal(first.text, "final-1");
    assert.equal(first.toolCalls.length, 1);
    assert.deepEqual(first.toolCalls[0], {
      toolCallId: "tool-1",
      toolName: "world_first_divergence",
      args: { root: "event-1" },
      result: { content: [{ type: "text", text: "evidence-1" }] },
      isError: false,
    });
    assert.ok(first.events.lastIndexOf("agent_end") < first.events.indexOf("agent_settled"));

    const second = await session.prompt("follow-up question", { timeoutMs: 2000 });
    assert.equal(session.id(), processId);
    assert.equal(second.requestId, "world-analyst-2");
    assert.equal(second.text, "final-2");
    assert.equal(second.toolCalls[0].isError, true);
  } finally {
    await session.shutdown();
  }
});

test("startup probe requires a selected model without consuming prompt correlation", async () => {
  const session = fakeSession(String.raw`
    let buffer = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      buffer += chunk;
      while (buffer.includes("\n")) {
        const at = buffer.indexOf("\n");
        const line = buffer.slice(0, at);
        buffer = buffer.slice(at + 1);
        if (!line) continue;
        const request = JSON.parse(line);
        if (request.type === "get_state") {
          emit({
            type: "response",
            id: request.id,
            command: "get_state",
            success: true,
            data: {
              model: { provider: "fake", id: "fake-model" },
              thinkingLevel: "off",
              isStreaming: false,
              isCompacting: false
            }
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
        emit({ type: "response", id: request.id, command: "prompt", success: true });
        emit({ type: "message_end", message: { role: "assistant", content: "after-probe" } });
        emit({ type: "agent_settled" });
      }
    });
    function emit(value) { process.stdout.write(JSON.stringify(value) + "\n"); }
  `);
  try {
    const probe = await session.probe({ timeoutMs: 2000 });
    assert.equal(probe.requestId, "world-analyst-probe-1");
    const result = await session.prompt("question", { timeoutMs: 2000 });
    assert.equal(result.requestId, "world-analyst-1");
    assert.equal(result.text, "after-probe");
  } finally {
    await session.shutdown();
  }
});

test("startup probe rejects a Pi session with no configured model", async () => {
  const session = fakeSession(String.raw`
    process.stdin.once("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response",
        id: request.id,
        command: "get_state",
        success: true,
        data: { model: null, thinkingLevel: "off", isStreaming: false, isCompacting: false }
      }) + "\n");
    });
    process.stdin.resume();
  `);
  try {
    await assert.rejects(
      session.probe({ timeoutMs: 2000 }),
      (error) =>
        error instanceof PiAnalystRpcCommandError && /no configured model/.test(error.message),
    );
  } finally {
    await session.shutdown();
  }
});

test("startup probe treats missing model state as protocol contamination", async () => {
  const session = fakeSession(String.raw`
    process.stdin.once("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response",
        id: request.id,
        command: "get_state",
        success: true,
        data: { thinkingLevel: "off", isStreaming: false, isCompacting: false }
      }) + "\n");
    });
    process.stdin.resume();
  `);

  await assert.rejects(
    session.probe({ timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcProtocolError && /omitted model state/.test(error.message),
  );
  await assert.rejects(
    session.probe({ timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcProtocolError,
  );
});

test("tool failures remain telemetry instead of transport failures", async () => {
  const session = fakeSession(REUSABLE_SERVER);
  try {
    await session.prompt("first", { timeoutMs: 2000 });
    const second = await session.prompt("second", { timeoutMs: 2000 });
    assert.equal(second.toolCalls[0].isError, true);
    assert.equal(second.text, "final-2");
  } finally {
    await session.shutdown();
  }
});

test("prompt rejection is a command error and the long-lived session remains usable", async () => {
  const session = fakeSession(String.raw`
    let buffer = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      buffer += chunk;
      while (buffer.includes("\n")) {
        const at = buffer.indexOf("\n");
        const line = buffer.slice(0, at);
        buffer = buffer.slice(at + 1);
        if (!line) continue;
        const request = JSON.parse(line);
        if (request.message === "reject") {
          emit({ type: "response", id: request.id, command: "prompt", success: false, error: "busy" });
        } else {
          emit({ type: "response", id: request.id, command: "prompt", success: true });
          emit({ type: "message_end", message: { role: "assistant", content: "accepted" } });
          emit({ type: "agent_settled" });
        }
      }
    });
    function emit(value) { process.stdout.write(JSON.stringify(value) + "\n"); }
  `);
  try {
    await assert.rejects(
      session.prompt("reject", { timeoutMs: 2000 }),
      (error) => error instanceof PiAnalystRpcCommandError,
    );
    const result = await session.prompt("accept", { timeoutMs: 2000 });
    assert.equal(result.text, "accepted");
    assert.equal(result.requestId, "world-analyst-2");
  } finally {
    await session.shutdown();
  }
});

test("correlation violations stay protocol errors and terminate the contaminated session", async () => {
  const session = fakeSession(String.raw`
    process.stdin.once("data", () => {
      process.stdout.write(JSON.stringify({
        type: "response", id: "wrong-id", command: "prompt", success: true
      }) + "\n");
    });
    process.stdin.resume();
  `);

  await assert.rejects(
    session.prompt("question", { timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcProtocolError && /correlation mismatch/.test(error.message),
  );
  await assert.rejects(
    session.prompt("another question", { timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcProtocolError,
  );
});

test("timeout is a transport failure and kills the session", async () => {
  const session = fakeSession(String.raw`
    process.stdin.on("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response", id: request.id, command: "prompt", success: true
      }) + "\n");
    });
    process.stdin.resume();
  `);

  await assert.rejects(
    session.prompt("never settles", { timeoutMs: 50 }),
    (error) => error instanceof PiAnalystRpcTransportError && /timed out/.test(error.message),
  );
  assert.ok(session.child.exitCode !== null || session.child.signalCode !== null);
});

test("EOF before agent_settled is a transport failure", async () => {
  const session = fakeSession(String.raw`
    process.stdin.once("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response", id: request.id, command: "prompt", success: true
      }) + "\n");
      process.stdout.write(JSON.stringify({
        type: "message_end", message: { role: "assistant", content: "partial" }
      }) + "\n", () => process.exit(0));
    });
  `);

  await assert.rejects(
    session.prompt("question", { timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcTransportError,
  );
});

test("session enforces one outstanding analyst prompt", async () => {
  const session = fakeSession(String.raw`
    process.stdin.on("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response", id: request.id, command: "prompt", success: true
      }) + "\n");
      setTimeout(() => {
        process.stdout.write(JSON.stringify({
          type: "message_end", message: { role: "assistant", content: "done" }
        }) + "\n");
        process.stdout.write(JSON.stringify({ type: "agent_settled" }) + "\n");
      }, 100);
    });
  `);
  try {
    const first = session.prompt("first", { timeoutMs: 2000 });
    await assert.rejects(
      session.prompt("second", { timeoutMs: 2000 }),
      (error) => error instanceof PiAnalystRpcProtocolError && /single-flight/.test(error.message),
    );
    assert.equal((await first).text, "done");
  } finally {
    await session.shutdown();
  }
});

test("extension errors are retained as structured analyst telemetry", async () => {
  const session = fakeSession(String.raw`
    process.stdin.once("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      for (const record of [
        { type: "response", id: request.id, command: "prompt", success: true },
        { type: "extension_error", extensionPath: "world-machine-analyst.mjs", event: "tool", error: "boom" },
        { type: "message_end", message: { role: "assistant", content: "degraded" } },
        { type: "agent_settled" },
      ]) process.stdout.write(JSON.stringify(record) + "\n");
    });
  `);
  try {
    const result = await session.prompt("question", { timeoutMs: 2000 });
    assert.equal(result.text, "degraded");
    assert.equal(result.extensionErrors.length, 1);
    assert.equal(result.extensionErrors[0].error, "boom");
  } finally {
    await session.shutdown();
  }
});
