import { spawn } from "node:child_process";
import { once } from "node:events";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_SHUTDOWN_TIMEOUT_MS = 1000;
const DEFAULT_PROMPT_TIMEOUT_MS = 120000;
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const RESTRICTED_LAUNCHER = resolve(REPO_ROOT, "scripts/run-pi-analyst.sh");

export class PiAnalystRpcError extends Error {
  constructor(message, details = {}) {
    super(message);
    this.name = "PiAnalystRpcError";
    this.details = details;
  }
}

export class PiAnalystRpcProtocolError extends PiAnalystRpcError {
  constructor(message, details = {}) {
    super(message, details);
    this.name = "PiAnalystRpcProtocolError";
  }
}

export class PiAnalystRpcCommandError extends PiAnalystRpcError {
  constructor(message, details = {}) {
    super(message, details);
    this.name = "PiAnalystRpcCommandError";
  }
}

export class PiAnalystRpcTransportError extends PiAnalystRpcError {
  constructor(message, details = {}) {
    super(message, details);
    this.name = "PiAnalystRpcTransportError";
  }
}

export class PiAnalystRpcSession {
  static spawnRestricted({
    leftArchive,
    rightArchive,
    provider,
    model,
    thinking,
    piProgram,
    analystProgram,
    env = process.env,
    cwd = REPO_ROOT,
    stderr = "inherit",
  }) {
    if (!leftArchive || !rightArchive) {
      throw new PiAnalystRpcTransportError("Pi analyst session requires both archive paths");
    }

    const args = [RESTRICTED_LAUNCHER, leftArchive, rightArchive];
    if (provider) args.push("--provider", provider);
    if (model) args.push("--model", model);
    if (thinking) args.push("--thinking", thinking);

    const childEnv = { ...env };
    if (piProgram) childEnv.PI_PROGRAM = piProgram;
    if (analystProgram) childEnv.WORLD_MACHINE_ANALYST_PROGRAM = analystProgram;

    const child = spawn("bash", args, {
      cwd,
      env: childEnv,
      stdio: ["pipe", "pipe", stderr],
    });
    return new PiAnalystRpcSession(child);
  }

  constructor(child) {
    if (!child.stdin || !child.stdout) {
      throw new PiAnalystRpcTransportError("Pi analyst RPC child must expose piped stdin/stdout");
    }

    this.child = child;
    this.buffer = Buffer.alloc(0);
    this.records = [];
    this.waiters = [];
    this.closedError = null;
    this.closed = false;
    this.busy = false;
    this.nextRequestNumber = 1;

    child.stdout.on("data", (chunk) => this.#acceptChunk(chunk));
    child.stdout.on("end", () => {
      if (!this.closed) {
        this.#finish(
          new PiAnalystRpcTransportError("Pi analyst RPC process closed stdout unexpectedly"),
        );
      }
    });
    child.on("error", (error) => {
      this.#finish(
        new PiAnalystRpcTransportError(`Pi analyst RPC process error: ${error.message}`, {
          cause: error,
        }),
      );
    });
    child.on("exit", (code, signal) => {
      if (!this.closed) {
        this.#finish(
          new PiAnalystRpcTransportError("Pi analyst RPC process exited unexpectedly", {
            code,
            signal,
          }),
        );
      }
    });
  }

  id() {
    return this.child.pid;
  }

  async prompt(message, { signal, timeoutMs = DEFAULT_PROMPT_TIMEOUT_MS } = {}) {
    if (typeof message !== "string" || message.length === 0) {
      throw new PiAnalystRpcProtocolError("Pi analyst prompt must be a non-empty string");
    }
    if (this.busy) {
      throw new PiAnalystRpcProtocolError("Pi analyst RPC session is single-flight");
    }
    if (this.closed || this.closedError) {
      throw this.closedError ?? new PiAnalystRpcTransportError("Pi analyst RPC session is closed");
    }
    if (signal?.aborted) {
      const error = new PiAnalystRpcTransportError("Pi analyst prompt aborted before dispatch");
      await this.#terminateAfterBrokenTurn(error);
      throw error;
    }

    const requestId = `world-analyst-${this.nextRequestNumber++}`;
    const turn = new TurnAccumulator(requestId);
    this.busy = true;

    try {
      await this.#write({ id: requestId, type: "prompt", message });
      return await this.#consumeTurn(turn, signal, timeoutMs);
    } catch (error) {
      if (!(error instanceof PiAnalystRpcCommandError)) {
        await this.#terminateAfterBrokenTurn(error);
      }
      throw error;
    } finally {
      this.busy = false;
    }
  }

  async shutdown(timeoutMs = DEFAULT_SHUTDOWN_TIMEOUT_MS) {
    if (this.closed) return;
    this.closed = true;

    if (this.child.exitCode !== null || this.child.signalCode !== null) {
      this.#finish(new PiAnalystRpcTransportError("Pi analyst RPC session closed"));
      return;
    }

    const exited = once(this.child, "exit");
    this.child.stdin.end();
    if (await settledBefore(exited, timeoutMs)) {
      this.#finish(new PiAnalystRpcTransportError("Pi analyst RPC session closed"));
      return;
    }

    const terminated = once(this.child, "exit");
    this.child.kill("SIGTERM");
    await settledBefore(terminated, timeoutMs);
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.child.kill("SIGKILL");
    }
    this.#finish(new PiAnalystRpcTransportError("Pi analyst RPC session closed"));
  }

  kill() {
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.child.kill("SIGTERM");
    }
  }

  async #consumeTurn(turn, signal, timeoutMs) {
    const deadline = timeoutMs > 0 ? Date.now() + timeoutMs : null;

    while (true) {
      const remaining = deadline === null ? null : deadline - Date.now();
      if (remaining !== null && remaining <= 0) {
        throw new PiAnalystRpcTransportError("Pi analyst prompt timed out", {
          requestId: turn.requestId,
          timeoutMs,
        });
      }

      const record = await this.#nextRecordWithInterruption(signal, remaining, turn.requestId);
      if (record.type === "response") {
        turn.acceptResponse(record);
        continue;
      }

      turn.acceptEvent(record);
      if (record.type === "agent_settled") {
        return turn.finish();
      }
    }
  }

  async #nextRecordWithInterruption(signal, timeoutMs, requestId) {
    const recordPromise = this.#nextRecord();
    if (signal?.aborted) {
      recordPromise.catch(() => {});
      throw new PiAnalystRpcTransportError("Pi analyst prompt aborted", {
        requestId,
      });
    }

    const races = [recordPromise];
    let onAbort;
    let timer;

    if (signal) {
      const aborted = new Promise((_, reject) => {
        onAbort = () => {
          reject(
            new PiAnalystRpcTransportError("Pi analyst prompt aborted", {
              requestId,
            }),
          );
        };
        signal.addEventListener("abort", onAbort, { once: true });
      });
      races.push(aborted);
    }

    if (timeoutMs !== null) {
      races.push(
        new Promise((_, reject) => {
          timer = setTimeout(() => {
            reject(
              new PiAnalystRpcTransportError("Pi analyst prompt timed out", {
                requestId,
                timeoutMs,
              }),
            );
          }, Math.max(0, timeoutMs));
        }),
      );
    }

    try {
      return await Promise.race(races);
    } catch (error) {
      recordPromise.catch(() => {});
      throw error;
    } finally {
      if (onAbort) signal.removeEventListener("abort", onAbort);
      if (timer) clearTimeout(timer);
    }
  }

  async #write(value) {
    if (this.closedError || !this.child.stdin.writable) {
      throw this.closedError ?? new PiAnalystRpcTransportError("Pi analyst RPC stdin is not writable");
    }
    const line = `${JSON.stringify(value)}\n`;
    await new Promise((resolveWrite, rejectWrite) => {
      this.child.stdin.write(line, "utf8", (error) => {
        if (error) {
          rejectWrite(
            new PiAnalystRpcTransportError(`failed to write Pi analyst RPC request: ${error.message}`, {
              cause: error,
            }),
          );
        } else {
          resolveWrite();
        }
      });
    });
  }

  #nextRecord() {
    if (this.records.length > 0) return Promise.resolve(this.records.shift());
    if (this.closedError) return Promise.reject(this.closedError);
    return new Promise((resolveRecord, rejectRecord) => {
      this.waiters.push({ resolve: resolveRecord, reject: rejectRecord });
    });
  }

  #acceptChunk(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    while (true) {
      const newline = this.buffer.indexOf(0x0a);
      if (newline < 0) return;
      const raw = this.buffer.subarray(0, newline);
      this.buffer = this.buffer.subarray(newline + 1);
      const line = raw.toString("utf8").replace(/\r$/, "");
      if (line.length === 0) continue;

      let record;
      try {
        record = JSON.parse(line);
      } catch (error) {
        this.#finish(
          new PiAnalystRpcProtocolError(`invalid Pi analyst RPC JSON: ${error.message}`, {
            line,
          }),
        );
        return;
      }

      const waiter = this.waiters.shift();
      if (waiter) waiter.resolve(record);
      else this.records.push(record);
    }
  }

  #finish(error) {
    if (this.closedError) return;
    this.closedError = error;
    const waiters = this.waiters.splice(0);
    for (const waiter of waiters) waiter.reject(error);
  }

  async #terminateAfterBrokenTurn(error) {
    this.closed = true;
    this.#finish(error);
    if (this.child.exitCode !== null || this.child.signalCode !== null) return;

    const exited = once(this.child, "exit");
    this.child.kill("SIGTERM");
    await settledBefore(exited, DEFAULT_SHUTDOWN_TIMEOUT_MS);
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.child.kill("SIGKILL");
    }
  }
}

class TurnAccumulator {
  constructor(requestId) {
    this.requestId = requestId;
    this.acknowledged = false;
    this.finalAssistantText = null;
    this.toolCalls = [];
    this.toolCallById = new Map();
    this.extensionErrors = [];
    this.events = [];
  }

  acceptResponse(response) {
    if (this.acknowledged) {
      throw new PiAnalystRpcProtocolError("unexpected extra Pi RPC command response during analyst turn", {
        requestId: this.requestId,
        response,
      });
    }
    if (response.id !== this.requestId || response.command !== "prompt") {
      throw new PiAnalystRpcProtocolError("Pi analyst prompt acknowledgement correlation mismatch", {
        expectedId: this.requestId,
        actualId: response.id,
        command: response.command,
      });
    }
    if (response.success !== true) {
      throw new PiAnalystRpcCommandError("Pi analyst prompt was rejected", {
        requestId: this.requestId,
        response,
      });
    }
    this.acknowledged = true;
  }

  acceptEvent(event) {
    if (!this.acknowledged) {
      throw new PiAnalystRpcProtocolError("Pi analyst event arrived before prompt acknowledgement", {
        requestId: this.requestId,
        event,
      });
    }

    this.events.push(event.type ?? "unknown");
    switch (event.type) {
      case "message_end":
        this.#acceptAssistantMessage(event.message);
        break;
      case "turn_end":
        this.#acceptAssistantMessage(event.message);
        break;
      case "tool_execution_start":
        this.#toolStart(event);
        break;
      case "tool_execution_end":
        this.#toolEnd(event);
        break;
      case "extension_error":
        this.extensionErrors.push(event);
        break;
      default:
        break;
    }
  }

  finish() {
    if (!this.acknowledged) {
      throw new PiAnalystRpcProtocolError("Pi analyst session settled without prompt acknowledgement", {
        requestId: this.requestId,
      });
    }
    const unfinished = this.toolCalls.filter((call) => call.completed !== true);
    if (unfinished.length > 0) {
      throw new PiAnalystRpcProtocolError("Pi analyst session settled with unfinished tool calls", {
        requestId: this.requestId,
        toolCallIds: unfinished.map((call) => call.toolCallId),
      });
    }

    return {
      requestId: this.requestId,
      text: this.finalAssistantText,
      toolCalls: this.toolCalls.map(({ completed, ...call }) => call),
      extensionErrors: this.extensionErrors,
      events: this.events,
    };
  }

  #acceptAssistantMessage(message) {
    if (!message || message.role !== "assistant") return;
    const text = assistantText(message);
    if (text !== null) this.finalAssistantText = text;
  }

  #toolStart(event) {
    const toolCallId = requiredString(event.toolCallId, "toolCallId");
    const toolName = requiredString(event.toolName, "toolName");
    if (this.toolCallById.has(toolCallId)) {
      throw new PiAnalystRpcProtocolError("duplicate Pi analyst tool_execution_start", {
        requestId: this.requestId,
        toolCallId,
      });
    }
    const call = {
      toolCallId,
      toolName,
      args: event.args ?? null,
      result: null,
      isError: null,
      completed: false,
    };
    this.toolCalls.push(call);
    this.toolCallById.set(toolCallId, call);
  }

  #toolEnd(event) {
    const toolCallId = requiredString(event.toolCallId, "toolCallId");
    const toolName = requiredString(event.toolName, "toolName");
    const call = this.toolCallById.get(toolCallId);
    if (!call) {
      throw new PiAnalystRpcProtocolError("Pi analyst tool_execution_end has no matching start", {
        requestId: this.requestId,
        toolCallId,
      });
    }
    if (call.completed) {
      throw new PiAnalystRpcProtocolError("duplicate Pi analyst tool_execution_end", {
        requestId: this.requestId,
        toolCallId,
      });
    }
    if (call.toolName !== toolName) {
      throw new PiAnalystRpcProtocolError("Pi analyst tool name changed during execution", {
        requestId: this.requestId,
        toolCallId,
        expectedTool: call.toolName,
        actualTool: toolName,
      });
    }
    call.result = event.result ?? null;
    call.isError = event.isError === true;
    call.completed = true;
  }
}

function assistantText(message) {
  if (typeof message.content === "string") return message.content;
  if (!Array.isArray(message.content)) return null;
  const parts = message.content
    .filter((part) => part && part.type === "text" && typeof part.text === "string")
    .map((part) => part.text);
  return parts.length > 0 ? parts.join("") : null;
}

function requiredString(value, field) {
  if (typeof value !== "string" || value.length === 0) {
    throw new PiAnalystRpcProtocolError(`Pi analyst event requires string ${field}`);
  }
  return value;
}

async function settledBefore(promise, timeoutMs) {
  let timer;
  try {
    return await Promise.race([
      promise.then(() => true),
      new Promise((resolveTimeout) => {
        timer = setTimeout(() => resolveTimeout(false), timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}
