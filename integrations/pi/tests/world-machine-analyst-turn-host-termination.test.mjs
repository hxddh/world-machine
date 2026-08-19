import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));

test("SIGTERM aborts an in-flight turn and tears down the restricted Pi child", async () => {
  const temp = await mkdtemp(join(tmpdir(), "world-machine-m223-cancel-"));
  const fakePi = join(temp, "fake-pi.mjs");
  const readyFile = join(temp, "ready.txt");
  await writeFile(fakePi, `#!/usr/bin/env node
import { writeFileSync } from "node:fs";
let buffer = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const newline = buffer.indexOf(0x0a);
    if (newline < 0) break;
    const line = buffer.subarray(0, newline).toString("utf8");
    buffer = buffer.subarray(newline + 1);
    if (!line) continue;
    const request = JSON.parse(line);
    process.stdout.write(JSON.stringify({ type: "response", id: request.id, command: "prompt", success: true }) + "\\n");
    writeFileSync(process.env.FAKE_PI_READY_FILE, String(process.pid));
  }
});
setInterval(() => {}, 1000);
`);
  await chmod(fakePi, 0o755);

  const repoRoot = resolve(new URL("../../..", import.meta.url).pathname);
  const hostPath = resolve(repoRoot, "integrations/pi/world-machine-analyst-turn-host.mjs");
  const child = spawn(
    process.execPath,
    [hostPath, "left.world", "right.world", "--provider", "fake", "--model", "fake"],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        PI_PROGRAM: fakePi,
        WORLD_MACHINE_ANALYST_PROGRAM: process.execPath,
        FAKE_PI_READY_FILE: readyFile,
      },
      stdio: ["pipe", "pipe", "pipe"],
    },
  );
  const exit = new Promise((resolveExit) => child.on("exit", (code, signal) => resolveExit({ code, signal })));

  try {
    child.stdin.write(`${JSON.stringify({ id: "ask-1", op: "ask", prompt: "hang" })}\n`);
    await waitForFile(readyFile);
    const piPid = Number(await readFile(readyFile, "utf8"));
    assert.ok(Number.isInteger(piPid) && piPid > 0);

    const cancelledAt = Date.now();
    assert.equal(child.kill("SIGTERM"), true);
    const [stdout, stderr, status] = await Promise.all([
      collect(child.stdout),
      collect(child.stderr),
      exit,
    ]);

    assert.ok(Date.now() - cancelledAt < 5000, "cancelled analyst host should exit promptly");
    assert.equal(status.code, 1, stderr);
    const responses = stdout.trim().split("\n").filter(Boolean).map((line) => JSON.parse(line));
    assert.equal(responses.length, 1);
    assert.equal(responses[0].id, "ask-1");
    assert.equal(responses[0].type, "error");
    assert.equal(responses[0].error.kind, "transport");
    assert.equal(responses[0].error.fatal, true);
    assert.match(responses[0].error.message, /aborted/i);
    assert.equal(processExists(piPid), false, "restricted Pi child must not survive host cancellation");
  } finally {
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
    await rm(temp, { recursive: true, force: true });
  }
});

async function waitForFile(path) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (existsSync(path)) return;
    await sleep(20);
  }
  throw new Error(`timed out waiting for ${path}`);
}

function processExists(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error?.code === "ESRCH") return false;
    throw error;
  }
}

async function collect(stream) {
  let output = "";
  stream.setEncoding("utf8");
  for await (const chunk of stream) output += chunk;
  return output;
}
