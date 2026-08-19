import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import test from "node:test";
import {
  PiAnalystRpcSession,
  PiAnalystRpcTransportError,
} from "../world-machine-analyst-rpc.mjs";

function hangingSession() {
  const child = spawn(process.execPath, ["-e", String.raw`
    process.stdin.on("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response", id: request.id, command: "prompt", success: true
      }) + "\n");
    });
    process.stdin.resume();
  `], { stdio: ["pipe", "pipe", "pipe"] });
  return { child, session: new PiAnalystRpcSession(child) };
}

test("abort terminates the active analyst RPC session", async () => {
  const { child, session } = hangingSession();
  const controller = new AbortController();
  const pending = session.prompt("abort me", {
    signal: controller.signal,
    timeoutMs: 2000,
  });
  setTimeout(() => controller.abort(), 25);

  await assert.rejects(
    pending,
    (error) =>
      error instanceof PiAnalystRpcTransportError && /aborted/.test(error.message),
  );
  assert.ok(child.exitCode !== null || child.signalCode !== null);
});

test("abort immediately after dispatch cannot slip past event listener setup", async () => {
  const { child, session } = hangingSession();
  const controller = new AbortController();
  const pending = session.prompt("race abort", {
    signal: controller.signal,
    timeoutMs: 2000,
  });
  queueMicrotask(() => controller.abort());

  await assert.rejects(
    pending,
    (error) =>
      error instanceof PiAnalystRpcTransportError && /aborted/.test(error.message),
  );
  assert.ok(child.exitCode !== null || child.signalCode !== null);
});
