import process from "node:process";
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

  async handle(request) {
    const parsed = parseRequest(request);
    try {
      const turn = await this.session.prompt(parsed.prompt, {
        timeoutMs: parsed.timeoutMs,
      });
      return envelope({
        type: "result",
        id: parsed.id,
        turn: normalizeTurn(turn),
      });
    } catch (error) {
      const mapped = mapSessionError(error);
      const response = envelope({
        type: "error",
        id: parsed.id,
        error: mapped.error,
      });
      return {
        response,
        fatal: mapped.fatal,
      };
    }
  }

  async shutdown() {
    await this.session.shutdown();
  }
}

export async function runAnalystTurnHost({
  stdin = process.stdin,
  stdout = process.stdout,
  stderr = process.stderr,
  argv = process.argv.slice(2),
  env = process.env,
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

  try {
    for await (const line of jsonLines(stdin)) {
      let request;
      try {
        request = JSON.parse(line);
      } catch (error) {
        throw new AnalystTurnHostInputError(`invalid analyst turn JSON: ${error.message}`);
      }

      const handled = await host.handle(request);
      const response = handled.response ?? handled;
      await writeJsonLine(stdout, response);
      if (handled.fatal === true) {
        throw new Error(response.error?.message ?? "fatal analyst session failure");
      }
    }
  } finally {
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
    tool_calls: turn.toolCalls ?? [],
    extension_errors: turn.extensionErrors ?? [],
    events: turn.events ?? [],
  };
}

function mapSessionError(error) {
  if (error instanceof PiAnalystRpcCommandError) {
    return {
      fatal: false,
      error: {
        kind: "command",
        message: error.message,
        details: error.details ?? {},
      },
    };
  }
  if (error instanceof PiAnalystRpcProtocolError) {
    return {
      fatal: true,
      error: {
        kind: "protocol",
        message: error.message,
        details: error.details ?? {},
      },
    };
  }
  if (error instanceof PiAnalystRpcTransportError) {
    return {
      fatal: true,
      error: {
        kind: "transport",
        message: error.message,
        details: error.details ?? {},
      },
    };
  }
  return {
    fatal: true,
    error: {
      kind: "internal",
      message: error instanceof Error ? error.message : String(error),
      details: {},
    },
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
  for await (const chunk of stream) {
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

async function writeJsonLine(stream, value) {
  const line = `${JSON.stringify(value)}\n`;
  await new Promise((resolveWrite, rejectWrite) => {
    stream.write(line, "utf8", (error) => {
      if (error) rejectWrite(error);
      else resolveWrite();
    });
  });
}

const isMain = process.argv[1] && new URL(import.meta.url).pathname === process.argv[1];
if (isMain) {
  runAnalystTurnHost().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
