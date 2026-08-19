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
      const options = { timeoutMs: parsed.timeoutMs };
      if (signal !== undefined) options.signal = signal;
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
} = {}) {
  const config = parseProcessArgs(argv);
  const session = PiAnalystRpcSession.spawnRestricted({
    leftArchive: config.leftArchive,
    rightArchive: config.rightArchive,
    provider: config.provider,
    model: config.model,
    thinking: config.thinking,
    env,
  });
  const host = new AnalystTurnHost(session);
  const abortController = new AbortController();
  const onTerminate = () => abortController.abort();
  signalSource.on("SIGTERM", onTerminate);

  try {
    for await (const line of jsonLines(stdin)) {
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
  const allowed = new Set(["id", "op", "prompt", "timeout_ms"]);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw new AnalystTurnHostInputError(`unknown analyst turn request field: ${key}`);
    }
  }
  if (value.op !== "ask") {
    throw new AnalystTurnHostInputError("analyst turn request op must be `ask`");
  }
  if (typeof value.id !== "string" || value.id.length === 0) {
    throw new AnalystTurnHostInputError("analyst turn request id must be a non-empty string");
  }
  if (typeof value.prompt !== "string" || value.prompt.length === 0) {
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

async function* jsonLines(stream) {
  let buffer = Buffer.alloc(0);
  for await (const chunk of stdinChunks(stream)) {
    buffer = Buffer.concat([buffer, Buffer.from(chunk)]);
    while (true) {
      const newline = buffer.indexOf(0x0a);
      if (newline < 0) break;
      const raw = buffer.subarray(0, newline);
      buffer = buffer.subarray(newline + 1);
      const line = raw.toString("utf8").replace(/\r$/, "");
      if (line.length > 0) yield line;
    }
  }
  if (buffer.length > 0) {
    const line = buffer.toString("utf8").replace(/\r$/, "");
    if (line.length > 0) yield line;
  }
}

async function* stdinChunks(stream) {
  for await (const chunk of stream) yield chunk;
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
