import { spawn } from "node:child_process";
import { once } from "node:events";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_SHUTDOWN_TIMEOUT_MS = 1000;
const DEFAULT_PROBE_TIMEOUT_MS = 10000;
const DEFAULT_PROMPT_TIMEOUT_MS = 120000;
const DEFAULT_MAX_RPC_RECORD_BYTES = 64 * 1024 * 1024;
const ANALYST_READY_COMMAND = "world-machine-analyst-ready";
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

  constructor(child, { maxRecordBytes = DEFAULT_MAX_RPC_RECORD_BYTES } = {}) {
    if (!child.stdin || !child.stdout) {
      throw new PiAnalystRpcTransportError("Pi analyst RPC child must expose piped stdin/stdout");
    }
    if (
      !Number.isSafeInteger(maxRecordBytes) ||
      maxRecordBytes <= 0 ||
      maxRecordBytes >= Number.MAX_SAFE_INTEGER
    ) {
      throw new PiAnalystRpcProtocolError(
        "Pi analyst RPC max record bytes must be a positive safe integer with framing headroom",
      );
    }

    this.child = child;
    this.maxRecordBytes = maxRecordBytes;
    this.recordBuffer = Buffer.alloc(0);
    this.recordBytes = 0;
    this.records = [];
    this.waiters = [];
    this.closedError = null;
    this.closed = false;
    this.terminationPromise = null;
    this.busy = false;
    this.nextRequestNumber = 1;
    this.nextProbeNumber = 1;

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

  async probe({ signal, timeoutMs = DEFAULT_PROBE_TIMEOUT_MS } = {}) {
    if (this.busy) {
      throw new PiAnalystRpcProtocolError("Pi analyst RPC session is single-flight");
    }
    if (this.closed || this.closedError) {
      throw this.closedError ?? new PiAnalystRpcTransportError("Pi analyst RPC session is closed");
    }
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
      throw new PiAnalystRpcProtocolError(
        "Pi analyst probe timeout must be a positive safe integer",
      );
    }
    if (signal?.aborted) {
      const error = new PiAnalystRpcTransportError("Pi analyst probe aborted before dispatch");
      await this.#terminateAfterBrokenTurn(error);
      throw error;
    }

    const probeId = `world-analyst-probe-${this.nextProbeNumber++}`;
    const deadline = Date.now() + timeoutMs;
    this.busy = true;
    try {
      const state = await this.#probeCommand(
        { id: `${probeId}-state`, type: "get_state" },
        "get_state",
        signal,
        deadline,
        probeId,
      );
      if (!state.data || typeof state.data !== "object" || Array.isArray(state.data)) {
        throw new PiAnalystRpcProtocolError("Pi analyst get_state probe returned invalid state", {
          requestId: probeId,
          response: state,
        });
      }
      if (!Object.prototype.hasOwnProperty.call(state.data, "model")) {
        throw new PiAnalystRpcProtocolError("Pi analyst get_state probe omitted model state", {
          requestId: probeId,
          response: state,
        });
      }
      if (state.data.model === null) {
        throw new PiAnalystRpcCommandError(
          "Pi analyst has no configured model. Configure a model and authentication in Pi, then recheck World Analyst.",
          { requestId: probeId },
        );
      }
      const model = state.data.model;
      if (
        typeof model !== "object" ||
        Array.isArray(model) ||
        typeof model.provider !== "string" ||
        model.provider.trim().length === 0 ||
        typeof model.id !== "string" ||
        model.id.trim().length === 0
      ) {
        throw new PiAnalystRpcProtocolError("Pi analyst get_state probe returned invalid model state", {
          requestId: probeId,
          response: state,
        });
      }

      const commands = await this.#probeCommand(
        { id: `${probeId}-commands`, type: "get_commands" },
        "get_commands",
        signal,
        deadline,
        probeId,
      );
      const entries = commands.data?.commands;
      if (!Array.isArray(entries)) {
        throw new PiAnalystRpcProtocolError("Pi analyst get_commands probe returned invalid commands", {
          requestId: probeId,
          response: commands,
        });
      }
      const ready = entries.some(
        (command) =>
          command &&
          command.name === ANALYST_READY_COMMAND &&
          command.source === "extension",
      );
      if (!ready) {
        throw new PiAnalystRpcProtocolError(
          "Pi analyst extension did not expose the World Machine readiness marker",
          { requestId: probeId },
        );
      }
      return { requestId: probeId };
    } catch (error) {
      if (!(error instanceof PiAnalystRpcCommandError)) {
        await this.#terminateAfterBrokenTurn(error);
      }
      throw error;
    } finally {
      this.busy = false;
    }
  }

  async #probeCommand(request, expectedCommand, signal, deadline, requestId) {
    const remaining = deadline - Date.now();
    if (remaining <= 0) {
      throw new PiAnalystRpcTransportError("Pi analyst probe timed out", { requestId });
    }
    await this.#write(request);
    const response = await this.#nextRecordWithInterruption(
      signal,
      remaining,
      requestId,
      "probe",
    );
    if (!response || response.type !== "response") {
      throw new PiAnalystRpcProtocolError(
        `Pi analyst probe received an event before ${expectedCommand} response`,
        { requestId, response },
      );
    }
    if (response.id !== request.id || response.command !== expectedCommand) {
      throw new PiAnalystRpcProtocolError("Pi analyst probe correlation mismatch", {
        expectedId: request.id,
        actualId: response.id,
        expectedCommand,
        command: response.command,
      });
    }
    if (response.success !== true) {
      throw new PiAnalystRpcCommandError(`Pi analyst ${expectedCommand} probe was rejected`, {
        requestId,
        response,
      });
    }
    return response;
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

  async #nextRecordWithInterruption(signal, timeoutMs, requestId, operation = "prompt") {
    const recordPromise = this.#nextRecord();
    if (signal?.aborted) {
      recordPromise.catch(() => {});
      throw new PiAnalystRpcTransportError(`Pi analyst ${operation} aborted`, {
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
            new PiAnalystRpcTransportError(`Pi analyst ${operation} aborted`, {
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
              new PiAnalystRpcTransportError(`Pi analyst ${operation} timed out`, {
                requestId,
                timeoutMs,
              }),
            );
          }, Math.max(0, timeoutMs));
        }),
      );
    }

    try {
      const record = await Promise.race(races);
      if (this.closedError) throw this.closedError;
      return record;
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
    if (this.closedError) return;
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    let offset = 0;

    while (offset < bytes.length) {
      const newline = bytes.indexOf(0x0a, offset);
      if (newline < 0) {
        this.#appendRecordFragment(bytes.subarray(offset));
        return;
      }

      if (!this.#appendRecordFragment(bytes.subarray(offset, newline))) return;
      const payload = this.#takeRecordPayload();
      if (payload === null) return;
      if (payload.length > 0 && !this.#acceptRecordPayload(payload)) return;
      offset = newline + 1;
    }
  }

  #appendRecordFragment(fragment) {
    const maxRawBytes = this.maxRecordBytes + 1;
    const totalBytes = this.recordBytes + fragment.length;
    if (totalBytes > maxRawBytes) {
      this.#failOversizedRecord();
      return false;
    }

    if (totalBytes === maxRawBytes) {
      const lastByte =
        fragment.length > 0
          ? fragment[fragment.length - 1]
          : this.recordBytes > 0
            ? this.recordBuffer[this.recordBytes - 1]
            : null;
      if (lastByte !== 0x0d) {
        this.#failOversizedRecord();
        return false;
      }
    }

    if (fragment.length === 0) return true;
    this.#ensureRecordCapacity(totalBytes);
    fragment.copy(this.recordBuffer, this.recordBytes);
    this.recordBytes = totalBytes;
    return true;
  }

  #ensureRecordCapacity(requiredBytes) {
    if (this.recordBuffer.length >= requiredBytes) return;
    const maxRawBytes = this.maxRecordBytes + 1;
    let nextCapacity =
      this.recordBuffer.length === 0 ? Math.min(4096, maxRawBytes) : this.recordBuffer.length;
    while (nextCapacity < requiredBytes) {
      nextCapacity = Math.min(maxRawBytes, Math.max(requiredBytes, nextCapacity * 2));
    }

    const next = Buffer.allocUnsafe(nextCapacity);
    if (this.recordBytes > 0) {
      this.recordBuffer.copy(next, 0, 0, this.recordBytes);
    }
    this.recordBuffer = next;
  }

  #takeRecordPayload() {
    const raw = this.recordBuffer.subarray(0, this.recordBytes);
    this.recordBuffer = Buffer.alloc(0);
    this.recordBytes = 0;
    const payload = raw.length > 0 && raw[raw.length - 1] === 0x0d ? raw.subarray(0, -1) : raw;
    if (payload.length > this.maxRecordBytes) {
      this.#failOversizedRecord();
      return null;
    }
    return payload;
  }

  #acceptRecordPayload(payload) {
    const line = payload.toString("utf8");
    let record;
    try {
      record = JSON.parse(line);
    } catch (error) {
      this.#finish(
        new PiAnalystRpcProtocolError(`invalid Pi analyst RPC JSON: ${error.message}`, {
          line,
        }),
      );
      return false;
    }

    const waiter = this.waiters.shift();
    if (waiter) waiter.resolve(record);
    else this.records.push(record);
    return true;
  }

  #failOversizedRecord() {
    const error = new PiAnalystRpcProtocolError(
      `Pi analyst RPC record exceeded the ${this.maxRecordBytes}-byte transport limit`,
      { maxRecordBytes: this.maxRecordBytes },
    );
    this.records.length = 0;
    void this.#terminateAfterBrokenTurn(error);
  }

  #finish(error) {
    if (this.closedError) return;
    this.recordBuffer = Buffer.alloc(0);
    this.recordBytes = 0;
    this.closedError = error;
    const waiters = this.waiters.splice(0);
    for (const waiter of waiters) waiter.reject(error);
  }

  #terminateAfterBrokenTurn(error) {
    if (this.terminationPromise) return this.terminationPromise;
    this.closed = true;
    this.#finish(error);
    this.terminationPromise = (async () => {
      if (this.child.exitCode !== null || this.child.signalCode !== null) return;

      const exited = once(this.child, "exit");
      this.child.kill("SIGTERM");
      await settledBefore(exited, DEFAULT_SHUTDOWN_TIMEOUT_MS);
      if (this.child.exitCode === null && this.child.signalCode === null) {
        this.child.kill("SIGKILL");
      }
    })().catch(() => {});
    return this.terminationPromise;
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
