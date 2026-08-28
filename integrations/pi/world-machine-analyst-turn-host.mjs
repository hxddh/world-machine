import process from "node:process";
import { fileURLToPath } from "node:url";
import {
  PiAnalystRpcCommandError,
  PiAnalystRpcProtocolError,
  PiAnalystRpcSession,
  PiAnalystRpcTransportError,
} from "./world-machine-analyst-rpc.mjs";

export const ANALYST_TURN_PROTOCOL = "world-machine-analyst-turns";
export const ANALYST_TURN_PROTOCOL_VERSION = 1;

const DEFAULT_MAX_ANALYST_TURN_INPUT_RECORD_BYTES = 64 * 1024 * 1024;

export class AnalystTurnHostInputError extends Error {
  constructor(message) {
    super(message);
    this.name = "AnalystTurnHostInputError";
  }
}

export class AnalystTurnHost {
  constructor(session) {
    this.session = session;
  }

  async handle(request, { signal } = {}) {
  const parsed = parseRequest(request);
  try {
    const options = {};
    if (parsed.timeoutMs !== undefined) options.timeoutMs = parsed.timeoutMs;
    if (signal !== undefined) options.signal = signal;
    if (parsed.op === "probe") {
      await this.session.probe(options);
      return envelope({ type: "ready", id: parsed.id });
    }
    const turn = await this.session.prompt(parsed.prompt, options);
    return envelope({
      type: "result",
      id: parsed.id,
      turn: normalizeTurn(turn),
    });
  } catch (error) {
    return envelope({
      type: "error",
      id: parsed.id,
      error: mapSessionError(error),
    });
  }
}

  async shutdown() {
    await this.session.shutdown();
  }
}

export async function runAnalystTurnHost({
  stdin = process.stdin,
  stdout = process.stdout,
  argv = process.argv.slice(2),
  env = process.env,
  signalSource = process,
  maxInputRecordBytes = DEFAULT_MAX_ANALYST_TURN_INPUT_RECORD_BYTES,
} = {}) {
  validateInputRecordLimit(maxInputRecordBytes);
  const config = parseProcessArgs(argv);
  const session = spawnRestrictedSession(config, env);
  const host = new AnalystTurnHost(session);
  const abortController = new AbortController();
  const onTerminate = () => {
    abortController.abort();
    if (typeof stdin.destroy === "function") stdin.destroy();
  };
  signalSource.on("SIGTERM", onTerminate);

  try {
    for await (const line of readAnalystTurnLines(stdin, maxInputRecordBytes)) {
      let request;
      try {
        request = JSON.parse(line);
      } catch (error) {
        throw new AnalystTurnHostInputError(`invalid analyst turn JSON: ${error.message}`);
      }

      const response = await host.handle(request, { signal: abortController.signal });
      await writeJsonLine(stdout, response);
      if (response.type === "error" && response.error.fatal === true) {
        throw new Error(response.error.message);
      }
    }
  } finally {
    signalSource.off("SIGTERM", onTerminate);
    await host.shutdown();
  }

  return 0;
}

function parseRequest(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new AnalystTurnHostInputError("analyst turn request must be a JSON object");
  }
  if (value.op !== "ask" && value.op !== "probe") {
    throw new AnalystTurnHostInputError("analyst turn request op must be `ask` or `probe`");
  }
  const allowed = value.op === "ask"
    ? new Set(["id", "op", "prompt", "timeout_ms"])
    : new Set(["id", "op", "timeout_ms"]);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw new AnalystTurnHostInputError(`unknown analyst turn request field: ${key}`);
    }
  }
  if (typeof value.id !== "string" || value.id.length === 0) {
    throw new AnalystTurnHostInputError("analyst turn request id must be a non-empty string");
  }
  if (value.op === "ask" && (typeof value.prompt !== "string" || value.prompt.length === 0)) {
    throw new AnalystTurnHostInputError("analyst turn request prompt must be a non-empty string");
  }
  let timeoutMs;
  if (value.timeout_ms !== undefined) {
    if (!Number.isSafeInteger(value.timeout_ms) || value.timeout_ms <= 0) {
      throw new AnalystTurnHostInputError("analyst turn timeout_ms must be a positive integer");
    }
    timeoutMs = value.timeout_ms;
  }
  return {
    id: value.id,
    op: value.op,
    prompt: value.prompt,
    timeoutMs,
  };
}

function parseProcessArgs(argv) {
  if (argv.length < 2) {
    throw new AnalystTurnHostInputError(
      "usage: world-machine-analyst-turn-host.mjs <left.world> <right.world> [--provider NAME] [--model NAME] [--thinking LEVEL]",
    );
  }

  const [leftArchive, rightArchive, ...rest] = argv;
  const config = { leftArchive, rightArchive };
  for (let index = 0; index < rest.length; index += 1) {
    const flag = rest[index];
    if (!["--provider", "--model", "--thinking"].includes(flag)) {
      throw new AnalystTurnHostInputError(`unknown analyst turn host argument: ${flag}`);
    }
    const value = rest[index + 1];
    if (!value || value.startsWith("--")) {
      throw new AnalystTurnHostInputError(`missing value for ${flag}`);
    }
    index += 1;
    if (flag === "--provider") config.provider = value;
    if (flag === "--model") config.model = value;
    if (flag === "--thinking") config.thinking = value;
  }
  return config;
}

function normalizeTurn(turn) {
  return {
    request_id: turn.requestId,
    text: turn.text ?? null,
    tool_calls: (turn.toolCalls ?? []).map(normalizeToolCall),
    runtime_errors: (turn.extensionErrors ?? []).map(normalizeRuntimeError),
  };
}

function normalizeToolCall(call) {
  const details = call.result?.details;
  const hasCanonicalOutput =
    details && Object.prototype.hasOwnProperty.call(details, "output");
  return {
    call_id: call.toolCallId,
    tool: details?.worldMachineTool ?? call.toolName,
    input: call.args ?? null,
    output: hasCanonicalOutput ? details.output : (call.result ?? null),
    is_error: call.isError === true,
  };
}

function normalizeRuntimeError(error) {
  return {
    kind: "extension",
    source: error.extensionPath ?? null,
    message: typeof error.error === "string" ? error.error : "extension error",
  };
}

function mapSessionError(error) {
  if (error instanceof PiAnalystRpcCommandError) {
    return {
      kind: "command",
      fatal: false,
      message: error.message,
    };
  }
  if (error instanceof PiAnalystRpcProtocolError) {
    return {
      kind: "protocol",
      fatal: true,
      message: error.message,
    };
  }
  if (error instanceof PiAnalystRpcTransportError) {
    return {
      kind: "transport",
      fatal: true,
      message: error.message,
    };
  }
  return {
    kind: "internal",
    fatal: true,
    message: error instanceof Error ? error.message : String(error),
  };
}

function envelope(body) {
  return {
    protocol: ANALYST_TURN_PROTOCOL,
    version: ANALYST_TURN_PROTOCOL_VERSION,
    ...body,
  };
}

function spawnRestrictedSession(config, env) {
  return PiAnalystRpcSession.spawnRestricted({
    leftArchive: config.leftArchive,
    rightArchive: config.rightArchive,
    provider: config.provider,
    model: config.model,
    thinking: config.thinking,
    env,
  });
}

function validateInputRecordLimit(maxRecordBytes) {
  if (
    !Number.isSafeInteger(maxRecordBytes) ||
    maxRecordBytes <= 0 ||
    maxRecordBytes > DEFAULT_MAX_ANALYST_TURN_INPUT_RECORD_BYTES
  ) {
    throw new AnalystTurnHostInputError(
      `analyst turn input max record bytes must be an integer in 1..=${DEFAULT_MAX_ANALYST_TURN_INPUT_RECORD_BYTES}`,
    );
  }
}

export async function* readAnalystTurnLines(
  stream,
  maxRecordBytes = DEFAULT_MAX_ANALYST_TURN_INPUT_RECORD_BYTES,
) {
  validateInputRecordLimit(maxRecordBytes);
  const maxRawBytes = maxRecordBytes + 1;
  let recordBuffer = Buffer.alloc(0);
  let recordBytes = 0;

  const ensureCapacity = (requiredBytes) => {
    if (recordBuffer.length >= requiredBytes) return;
    let nextCapacity =
      recordBuffer.length === 0 ? Math.min(4096, maxRawBytes) : recordBuffer.length;
    while (nextCapacity < requiredBytes) {
      nextCapacity = Math.min(maxRawBytes, Math.max(requiredBytes, nextCapacity * 2));
    }
    const next = Buffer.allocUnsafe(nextCapacity);
    if (recordBytes > 0) recordBuffer.copy(next, 0, 0, recordBytes);
    recordBuffer = next;
  };

  const failOversized = () => {
    throw new AnalystTurnHostInputError(
      `analyst turn input record exceeded the ${maxRecordBytes}-byte transport limit`,
    );
  };

  const appendFragment = (fragment) => {
    const totalBytes = recordBytes + fragment.length;
    if (totalBytes > maxRawBytes) failOversized();
    if (totalBytes === maxRawBytes) {
      const lastByte =
        fragment.length > 0
          ? fragment[fragment.length - 1]
          : recordBytes > 0
            ? recordBuffer[recordBytes - 1]
            : null;
      if (lastByte !== 0x0d) failOversized();
    }
    if (fragment.length === 0) return;
    ensureCapacity(totalBytes);
    fragment.copy(recordBuffer, recordBytes);
    recordBytes = totalBytes;
  };

  const takePayload = () => {
    const raw = recordBuffer.subarray(0, recordBytes);
    recordBuffer = Buffer.alloc(0);
    recordBytes = 0;
    const payload = raw.length > 0 && raw[raw.length - 1] === 0x0d ? raw.subarray(0, -1) : raw;
    if (payload.length > maxRecordBytes) failOversized();
    return payload;
  };

  try {
    for await (const chunk of stream) {
      const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      let offset = 0;
      while (offset < bytes.length) {
        const newline = bytes.indexOf(0x0a, offset);
        if (newline < 0) {
          appendFragment(bytes.subarray(offset));
          break;
        }

        appendFragment(bytes.subarray(offset, newline));
        const payload = takePayload();
        if (payload.length > 0) yield payload.toString("utf8");
        offset = newline + 1;
      }
    }

    if (recordBytes > 0) {
      const payload = takePayload();
      if (payload.length > 0) yield payload.toString("utf8");
    }
  } finally {
    recordBuffer = Buffer.alloc(0);
    recordBytes = 0;
  }
}

async function writeJsonLine(stream, value) {
  const line = `${JSON.stringify(value)}\n`;
  await new Promise((resolveWrite, rejectWrite) => {
    stream.write(line, "utf8", (error) => {
      if (error) rejectWrite(error);
      else resolveWrite();
    });
  });
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  runAnalystTurnHost().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
