#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const TURN_PROTOCOL = "world-machine-analyst-turns";
const TURN_PROTOCOL_VERSION = 1;
const TOOL_PROTOCOL_NAME = "world.first-divergence";

main().catch((error) => {
  process.stderr.write(`Packaged analyst runtime smoke failed: ${error.stack ?? error.message}\n`);
  process.exitCode = 1;
});

async function main() {
  const appArgument = process.argv[2];
  if (!appArgument) {
    throw new Error(
      "usage: node scripts/check-packaged-analyst-runtime.mjs <World Machine.app>",
    );
  }

  const appDir = resolve(appArgument);
  const runtimeRoot = join(appDir, "Contents", "Resources", "Analyst Runtime");
  const runtime = {
    root: runtimeRoot,
    turnHost: join(runtimeRoot, "integrations", "pi", "world-machine-analyst-turn-host.mjs"),
    rpc: join(runtimeRoot, "integrations", "pi", "world-machine-analyst-rpc.mjs"),
    extension: join(runtimeRoot, "integrations", "pi", "world-machine-analyst.mjs"),
    client: join(runtimeRoot, "integrations", "pi", "world-machine-analyst-client.mjs"),
    launcher: join(runtimeRoot, "scripts", "run-pi-analyst.sh"),
    toolHost: join(runtimeRoot, "bin", "world-agent-tool-stdio"),
  };
  await validateRuntime(runtime);

  const temp = await mkdtemp(join(tmpdir(), "world-machine-m224-"));
  try {
    const workDir = join(temp, "outside-source-checkout");
    await mkdir(workDir);
    const leftArchive = join(temp, "left.world");
    const rightArchive = join(temp, "right.world");
    const fakePi = join(temp, "fake-pi.mjs");
    const fakePiLog = join(temp, "fake-pi.jsonl");

    await writeFile(leftArchive, `${JSON.stringify(archiveFixture(false), null, 2)}\n`);
    await writeFile(rightArchive, `${JSON.stringify(archiveFixture(true), null, 2)}\n`);
    await writeFile(fakePi, fakePiProgram());
    await chmod(fakePi, 0o755);

    assert.notEqual(
      resolve(workDir),
      resolve(runtimeRoot),
      "smoke driver must start outside the packaged runtime root",
    );

    const child = spawn(
      process.execPath,
      [
        runtime.turnHost,
        leftArchive,
        rightArchive,
        "--provider",
        "fake",
        "--model",
        "fake",
      ],
      {
        cwd: workDir,
        env: {
          ...process.env,
          PI_PROGRAM: fakePi,
          WORLD_MACHINE_ANALYST_PROGRAM: runtime.toolHost,
          WORLD_MACHINE_SMOKE_RUNTIME_ROOT: runtimeRoot,
          WORLD_MACHINE_SMOKE_LOG: fakePiLog,
        },
        stdio: ["pipe", "pipe", "pipe"],
      },
    );
    child.stdin.on("error", () => {});

    child.stdin.write(`${JSON.stringify({ id: "smoke-1", op: "ask", prompt: "first packaged turn" })}\n`);
    child.stdin.write(`${JSON.stringify({ id: "smoke-2", op: "ask", prompt: "second packaged turn" })}\n`);
    child.stdin.end();

    const stdoutPromise = collect(child.stdout);
    const stderrPromise = collect(child.stderr);
    const exitPromise = new Promise((resolveExit) => {
      child.on("exit", (code, signal) => resolveExit({ code, signal }));
      child.on("error", (error) => resolveExit({ code: null, signal: null, error }));
    });
    const timeout = setTimeout(() => child.kill("SIGKILL"), 20_000);
    const [{ code, signal, error }, stdout, stderr] = await Promise.all([
      exitPromise,
      stdoutPromise,
      stderrPromise,
    ]);
    clearTimeout(timeout);

    assert.equal(error, undefined, error?.message);
    assert.equal(signal, null, `packaged turn host was killed by ${signal}\n${stderr}`);
    assert.equal(code, 0, stderr);

    const responses = stdout
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    assert.equal(responses.length, 2, stdout);
    assertTurn(responses[0], 1);
    assertTurn(responses[1], 2);

    const serializedResponses = JSON.stringify(responses);
    assert.equal(
      serializedResponses.includes("world_first_divergence"),
      false,
      "provider-normalized Pi tool names must not escape the stable turn protocol",
    );
    assert.equal(
      serializedResponses.includes(runtimeRoot),
      false,
      "packaged filesystem paths must not escape the stable turn protocol",
    );

    const lifecycle = (await readFile(fakePiLog, "utf8"))
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    const starts = lifecycle.filter((entry) => entry.type === "start");
    const prompts = lifecycle.filter((entry) => entry.type === "prompt");
    const shutdowns = lifecycle.filter((entry) => entry.type === "shutdown");
    assert.equal(starts.length, 1, JSON.stringify(lifecycle));
    assert.equal(prompts.length, 2, JSON.stringify(lifecycle));
    assert.equal(shutdowns.length, 1, JSON.stringify(lifecycle));
    assert.equal(prompts[0].pid, starts[0].pid);
    assert.equal(prompts[1].pid, starts[0].pid);
    assert.equal(shutdowns[0].pid, starts[0].pid);
    assert.deepEqual(
      prompts.map((entry) => entry.turn),
      [1, 2],
      "both asks must reuse one long-lived Pi analyst process",
    );

    process.stdout.write(
      `Packaged analyst runtime smoke passed: two turns reused Pi pid ${starts[0].pid}.\n`,
    );
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}

async function validateRuntime(runtime) {
  await requireFile(runtime.turnHost, "turn host");
  await requireFile(runtime.rpc, "RPC module");
  await requireFile(runtime.extension, "Pi extension");
  await requireFile(runtime.client, "tool client");
  await requireFile(runtime.launcher, "restricted launcher", true);
  await requireFile(runtime.toolHost, "read-only tool host", true);
}

async function requireFile(path, label, executable = false) {
  let metadata;
  try {
    metadata = await stat(path);
  } catch (error) {
    throw new Error(`packaged analyst ${label} is missing: ${path}`, { cause: error });
  }
  if (!metadata.isFile() || metadata.size === 0) {
    throw new Error(`packaged analyst ${label} is missing or empty: ${path}`);
  }
  if (executable && (metadata.mode & 0o111) === 0) {
    throw new Error(`packaged analyst ${label} is not executable: ${path}`);
  }
}

function archiveFixture(withDivergence) {
  const events = [
    {
      id: 1,
      kind: "storm_started",
      world_time: 1,
      actor: null,
      targets: [],
      caused_by: [],
      payload: {},
      changes: [],
    },
  ];
  if (withDivergence) {
    events.push({
      id: 2,
      kind: "order_lost",
      world_time: 2,
      actor: null,
      targets: [],
      caused_by: [1],
      payload: {},
      changes: [],
    });
  }
  return {
    format: "world-machine",
    format_version: 1,
    pack: {
      id: "world-machine.tiny-society",
      version: "0.1.0",
    },
    world_time: withDivergence ? 2 : 1,
    events,
    pending: [],
  };
}

function assertTurn(response, turn) {
  assert.equal(response.protocol, TURN_PROTOCOL);
  assert.equal(response.version, TURN_PROTOCOL_VERSION);
  assert.equal(response.type, "result");
  assert.equal(response.id, `smoke-${turn}`);
  assert.equal(response.turn.request_id, `world-analyst-${turn}`);
  assert.equal(response.turn.text, `packaged-answer-${turn}`);
  assert.deepEqual(response.turn.runtime_errors, []);
  assert.equal(response.turn.tool_calls.length, 1);

  const call = response.turn.tool_calls[0];
  assert.equal(call.call_id, `packaged-tool-${turn}`);
  assert.equal(call.tool, TOOL_PROTOCOL_NAME);
  assert.deepEqual(call.input, {
    root: "event-1",
    direction: "downstream",
    window_depth: 1,
    max_depth: 2,
  });
  assert.equal(call.is_error, false);
  assert.equal(call.output.root, "event-1");
  assert.equal(call.output.direction, "downstream");
  assert.equal(call.output.max_depth, 2);
  assert.equal(call.output.identical_within_depth, false);
  assert.equal(call.output.divergence_depth, 1);
  assert.equal(call.output.truncated, false);
  assert.ok(Array.isArray(call.output.witnesses));
  assert.ok(call.output.witnesses.length > 0, "expected a real divergence witness from the packaged Rust tool host");
}

async function collect(stream) {
  let output = "";
  stream.setEncoding("utf8");
  for await (const chunk of stream) output += chunk;
  return output;
}

function fakePiProgram() {
  return `#!/usr/bin/env node
import assert from "node:assert/strict";
import { appendFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";

main().catch((error) => {
  process.stderr.write(\`fake Pi harness failed: \${error.stack ?? error.message}\\n\`);
  process.exitCode = 1;
});

async function main() {
  const args = process.argv.slice(2);
  assert.equal(valueAfter(args, "--mode"), "rpc");
  for (const flag of [
    "--no-session",
    "--no-builtin-tools",
    "--no-extensions",
    "--no-skills",
    "--no-prompt-templates",
    "--no-themes",
    "--no-context-files",
  ]) {
    assert.ok(args.includes(flag), \`packaged launcher omitted \${flag}\`);
  }

  const extensionPath = resolve(valueAfter(args, "--extension"));
  const runtimeRoot = resolve(requiredEnv("WORLD_MACHINE_SMOKE_RUNTIME_ROOT"));
  const logPath = requiredEnv("WORLD_MACHINE_SMOKE_LOG");
  assert.equal(
    extensionPath,
    join(runtimeRoot, "integrations", "pi", "world-machine-analyst.mjs"),
    "launcher must load the packaged Pi extension",
  );
  assert.equal(
    resolve(process.cwd()),
    runtimeRoot,
    "RPC must launch Pi from the packaged runtime root rather than a source checkout",
  );
  assert.equal(
    resolve(requiredEnv("WORLD_MACHINE_ANALYST_PROGRAM")),
    join(runtimeRoot, "bin", "world-agent-tool-stdio"),
    "extension must use the packaged Rust tool host",
  );

  const handlers = new Map();
  const tools = new Map();
  let activeTools = [];
  const pi = {
    on(event, handler) {
      const current = handlers.get(event) ?? [];
      current.push(handler);
      handlers.set(event, current);
    },
    setActiveTools(names) {
      activeTools = [...names];
    },
    registerTool(tool) {
      assert.equal(typeof tool?.name, "string");
      tools.set(tool.name, tool);
    },
  };

  const extension = (await import(pathToFileURL(extensionPath).href)).default;
  assert.equal(typeof extension, "function");
  extension(pi);
  await fire(handlers, "session_start");
  assert.ok(activeTools.includes("world_first_divergence"));
  const tool = tools.get("world_first_divergence");
  assert.ok(tool, "packaged extension did not register world_first_divergence");
  await log(logPath, { type: "start", pid: process.pid, cwd: process.cwd(), extension: extensionPath });

  let turn = 0;
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of lines) {
    if (!line) continue;
    const request = JSON.parse(line);
    assert.equal(request.type, "prompt");
    assert.equal(typeof request.id, "string");
    turn += 1;
    emit({ type: "response", id: request.id, command: "prompt", success: true });

    const callId = \`packaged-tool-\${turn}\`;
    const input = {
      root: "event-1",
      direction: "downstream",
      window_depth: 1,
      max_depth: 2,
    };
    emit({
      type: "tool_execution_start",
      toolCallId: callId,
      toolName: tool.name,
      args: input,
    });
    const result = await tool.execute(callId, input, new AbortController().signal);
    emit({
      type: "tool_execution_end",
      toolCallId: callId,
      toolName: tool.name,
      result,
      isError: false,
    });
    emit({
      type: "message_end",
      message: { role: "assistant", content: \`packaged-answer-\${turn}\` },
    });
    emit({ type: "agent_settled" });
    await log(logPath, { type: "prompt", pid: process.pid, turn, requestId: request.id });
  }

  await fire(handlers, "session_shutdown");
  await log(logPath, { type: "shutdown", pid: process.pid, turns: turn });
}

function valueAfter(args, flag) {
  const index = args.indexOf(flag);
  assert.notEqual(index, -1, \`missing \${flag}\`);
  const value = args[index + 1];
  assert.ok(value && !value.startsWith("--"), \`missing value for \${flag}\`);
  return value;
}

function requiredEnv(name) {
  const value = process.env[name];
  assert.ok(value, \`missing environment variable \${name}\`);
  return value;
}

async function fire(handlers, event) {
  for (const handler of handlers.get(event) ?? []) await handler();
}

async function log(path, value) {
  await appendFile(path, JSON.stringify(value) + "\\n", "utf8");
}

function emit(value) {
  process.stdout.write(JSON.stringify(value) + "\\n");
}
`;
}
