import { spawn } from "node:child_process";
import { once } from "node:events";

const DEFAULT_SHUTDOWN_TIMEOUT_MS = 500;
const DEFAULT_MAX_ANALYST_RECORD_BYTES = 64 * 1024 * 1024;

export const ANALYST_PROTOCOL = "world-machine-readonly-tools";
export const ANALYST_PROTOCOL_VERSION = 1;

export class AnalystBridgeError extends Error {
  constructor(message, details = {}) {
    super(message);
    this.name = "AnalystBridgeError";
    this.details = details;
  }
}

export class AnalystJsonlClient {
  static spawn(program, args, options = {}) {
    const child = spawn(program, args, {
      stdio: ["pipe", "pipe", options.stderr ?? "inherit"],
      env: options.env ?? process.env,
      cwd: options.cwd,
    });
    return new AnalystJsonlClient(child);
  }

  constructor(child, { maxRecordBytes = DEFAULT_MAX_ANALYST_RECORD_BYTES } = {}) {
    if (!child.stdin || !child.stdout) {
      throw new AnalystBridgeError("analyst child must expose piped stdin/stdout");
    }
    if (
      !Number.isSafeInteger(maxRecordBytes) ||
      maxRecordBytes <= 0 ||
      maxRecordBytes >= Number.MAX_SAFE_INTEGER
    ) {
      throw new AnalystBridgeError(
        "analyst JSONL max record bytes must be a positive safe integer with framing headroom",
      );
    }

    this.child = child;
    this.maxRecordBytes = maxRecordBytes;
    this.recordBuffer = Buffer.alloc(0);
    this.recordBytes = 0;
    this.lines = [];
    this.waiters = [];
    this.closedError = null;
    this.framingError = null;
    this.terminationPromise = null;
    this.busy = false;

    child.stdout.on("data", (chunk) => this.#acceptChunk(chunk));
    child.stdout.on("end", () => this.#finish(new AnalystBridgeError("analyst process closed stdout")));
    child.on("error", (error) =>
      this.#finish(new AnalystBridgeError(`analyst process error: ${error.message}`, { cause: error })),
    );
    child.on("exit", (code, signal) => {
      this.#finish(
        new AnalystBridgeError("analyst process exited", {
          code,
          signal,
        }),
      );
    });
  }

  async listTools(signal) {
    const envelope = await this.#roundTrip({ op: "list-tools" }, signal);
    if (envelope.type !== "catalog" || !Array.isArray(envelope.tools)) {
      throw new AnalystBridgeError(`unexpected analyst response type: ${String(envelope.type)}`, {
        expected: "catalog",
        actual: envelope.type,
      });
    }
    return envelope.tools;
  }

  async invoke(callId, tool, input, signal) {
    const envelope = await this.#roundTrip(
      {
        op: "invoke",
        call_id: callId,
        tool,
        input,
      },
      signal,
    );

    if (envelope.type !== "result" && envelope.type !== "error") {
      throw new AnalystBridgeError(`unexpected analyst response type: ${String(envelope.type)}`, {
        expected: "result-or-error",
        actual: envelope.type,
      });
    }
    if (envelope.call_id !== callId || envelope.tool !== tool) {
      throw new AnalystBridgeError("analyst response correlation mismatch", {
        expectedCallId: callId,
        expectedTool: tool,
        actualCallId: envelope.call_id,
        actualTool: envelope.tool,
      });
    }
    if (envelope.type === "error") {
      const kind = envelope.error?.kind ?? "unknown";
      const message = envelope.error?.message ?? "unknown analyst tool error";
      throw new AnalystBridgeError(`analyst tool ${tool} failed (${kind}): ${message}`, {
        callId,
        tool,
        remoteError: envelope.error,
      });
    }
    return envelope.output;
  }

  async shutdown() {
    if (this.child.exitCode !== null || this.child.signalCode !== null) {
      return;
    }
    const exited = once(this.child, "exit").then(() => true);
    this.child.stdin.end();
    const timedOut = new Promise((resolve) => {
      const timer = setTimeout(() => resolve(false), DEFAULT_SHUTDOWN_TIMEOUT_MS);
      timer.unref?.();
    });
    if (!(await Promise.race([exited, timedOut]))) {
      const terminated = once(this.child, "exit");
      this.child.kill("SIGTERM");
      await Promise.race([
        terminated,
        new Promise((resolve) => setTimeout(resolve, DEFAULT_SHUTDOWN_TIMEOUT_MS)),
      ]);
    }
  }

  kill() {
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.child.kill("SIGTERM");
    }
  }

  async #roundTrip(request, signal) {
    if (this.busy) {
      throw new AnalystBridgeError("analyst JSONL client is single-flight");
    }
    if (signal?.aborted) {
      throw new AnalystBridgeError("analyst tool call aborted before dispatch");
    }
    if (this.closedError) {
      throw this.closedError;
    }

    this.busy = true;
    try {
      const responsePromise = this.#nextLine();
      try {
        await this.#writeLine(request);
      } catch (error) {
        responsePromise.catch(() => {});
        if (this.framingError) throw this.framingError;
        throw error;
      }
      const line = await this.#withAbort(responsePromise, signal);
      if (this.framingError) throw this.framingError;

      let envelope;
      try {
        envelope = JSON.parse(line);
      } catch (error) {
        throw new AnalystBridgeError(`invalid analyst response JSON: ${error.message}`, { line });
      }
      if (envelope.protocol !== ANALYST_PROTOCOL) {
        throw new AnalystBridgeError(`unexpected analyst protocol: ${String(envelope.protocol)}`, {
          expected: ANALYST_PROTOCOL,
          actual: envelope.protocol,
        });
      }
      if (envelope.version !== ANALYST_PROTOCOL_VERSION) {
        throw new AnalystBridgeError(`unexpected analyst protocol version: ${String(envelope.version)}`, {
          expected: ANALYST_PROTOCOL_VERSION,
          actual: envelope.version,
        });
      }
      return envelope;
    } finally {
      this.busy = false;
    }
  }

  async #writeLine(value) {
    if (this.closedError || !this.child.stdin.writable) {
      throw this.closedError ?? new AnalystBridgeError("analyst process stdin is not writable");
    }
    const line = `${JSON.stringify(value)}\n`;
    await new Promise((resolve, reject) => {
      this.child.stdin.write(line, "utf8", (error) => {
        if (error) {
          reject(
            new AnalystBridgeError(`failed to write analyst request: ${error.message}`, {
              cause: error,
            }),
          );
        } else {
          resolve();
        }
      });
    });
  }

  #nextLine() {
    if (this.lines.length > 0) {
      return Promise.resolve(this.lines.shift());
    }
    if (this.closedError) {
      return Promise.reject(this.closedError);
    }
    return new Promise((resolve, reject) => {
      this.waiters.push({ resolve, reject });
    });
  }

  async #withAbort(promise, signal) {
    if (!signal) return promise;
    if (signal.aborted) {
      promise.catch(() => {});
      this.kill();
      throw new AnalystBridgeError("analyst tool call aborted");
    }
    let onAbort;
    const aborted = new Promise((_, reject) => {
      onAbort = () => {
        const error = new AnalystBridgeError("analyst tool call aborted");
        this.kill();
        reject(error);
      };
      signal.addEventListener("abort", onAbort, { once: true });
    });
    try {
      return await Promise.race([promise, aborted]);
    } finally {
      signal.removeEventListener("abort", onAbort);
    }
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
      if (payload.length > 0) this.#acceptRecordPayload(payload);
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
    const waiter = this.waiters.shift();
    if (waiter) waiter.resolve(line);
    else this.lines.push(line);
  }

  #failOversizedRecord() {
    const error = new AnalystBridgeError(
      `analyst response record exceeded the ${this.maxRecordBytes}-byte transport limit`,
      { kind: "framing-overflow", maxRecordBytes: this.maxRecordBytes },
    );
    this.lines.length = 0;
    this.framingError = error;
    void this.#terminateAfterFramingFailure(error);
  }

  #terminateAfterFramingFailure(error) {
    if (this.terminationPromise) return this.terminationPromise;
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

  #finish(error) {
    if (this.closedError) return;
    this.recordBuffer = Buffer.alloc(0);
    this.recordBytes = 0;
    this.closedError = error;
    const waiters = this.waiters.splice(0);
    for (const waiter of waiters) waiter.reject(error);
  }
}

export function providerSafeToolName(hostName) {
  let name = hostName.replace(/[^A-Za-z0-9_-]/g, "_").replace(/-+/g, "_");
  if (!/^[A-Za-z_]/.test(name)) name = `world_${name}`;
  if (name.length === 0) name = "world_tool";
  return name.slice(0, 64);
}

async function settledBefore(promise, timeoutMs) {
  let timer;
  try {
    return await Promise.race([
      promise.then(() => true, () => false),
      new Promise((resolve) => {
        timer = setTimeout(() => resolve(false), timeoutMs);
        timer.unref?.();
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}
