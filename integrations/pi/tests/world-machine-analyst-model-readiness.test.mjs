import assert from "node:assert/strict";
import { spawn } from "node:child_process";
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
  PiAnalystRpcSession,
} from "../world-machine-analyst-rpc.mjs";

function stateSession(stateData) {
  const script = String.raw`
    const stateData = ${JSON.stringify(stateData)};
    process.stdin.once("data", (chunk) => {
      const request = JSON.parse(chunk.toString("utf8").trim());
      process.stdout.write(JSON.stringify({
        type: "response",
        id: request.id,
        command: "get_state",
        success: true,
        data: stateData
      }) + "\n");
    });
    process.stdin.resume();
  `;
  const child = spawn(process.execPath, ["-e", script], {
    stdio: ["pipe", "pipe", "pipe"],
  });
  return new PiAnalystRpcSession(child);
}

test("startup probe fails closed on a structurally malformed selected model", async () => {
  const session = stateSession({
    model: { provider: "", id: "fake-model" },
    thinkingLevel: "off",
    isStreaming: false,
    isCompacting: false,
  });

  await assert.rejects(
    session.probe({ timeoutMs: 2000 }),
    (error) =>
      error instanceof PiAnalystRpcProtocolError &&
      /invalid model state/.test(error.message),
  );
  await assert.rejects(
    session.probe({ timeoutMs: 2000 }),
    (error) => error instanceof PiAnalystRpcProtocolError,
  );
});

test("turn host keeps a no-model startup rejection provider-neutral", async () => {
  const secretProvider = "provider-that-must-not-escape";
  const secretModel = "model-that-must-not-escape";
  const host = new AnalystTurnHost({
    async probe() {
      throw new PiAnalystRpcCommandError("Pi analyst has no configured model", {
        provider: secretProvider,
        model: secretModel,
      });
    },
    async prompt() {
      throw new Error("probe must not dispatch a model prompt");
    },
    async shutdown() {},
  });

  const response = await host.handle({ id: "probe-1", op: "probe", timeout_ms: 2000 });
  assert.deepEqual(response, {
    protocol: ANALYST_TURN_PROTOCOL,
    version: ANALYST_TURN_PROTOCOL_VERSION,
    type: "error",
    id: "probe-1",
    error: {
      kind: "command",
      fatal: false,
      message: "Pi analyst has no configured model",
    },
  });
  const serialized = JSON.stringify(response);
  assert.equal(serialized.includes(secretProvider), false);
  assert.equal(serialized.includes(secretModel), false);
});

test("probe request shape rejects prompt archive and tool fields", async () => {
  const host = new AnalystTurnHost({
    async probe() {},
    async prompt() {},
    async shutdown() {},
  });

  for (const extra of [
    { prompt: "not allowed" },
    { left_archive: "/tmp/other.world" },
    { tool: "world.first-divergence" },
  ]) {
    await assert.rejects(
      host.handle({ id: "probe-strict", op: "probe", timeout_ms: 1000, ...extra }),
      (error) =>
        error instanceof AnalystTurnHostInputError &&
        /unknown analyst turn request field/.test(error.message),
    );
  }
});
