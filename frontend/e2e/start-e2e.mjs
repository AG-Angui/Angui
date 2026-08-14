import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { resolve } from "node:path";

const workspaceRoot = resolve(import.meta.dirname, "../..");
const frontendRoot = resolve(import.meta.dirname, "..");
const e2eDirectory = resolve(workspaceRoot, ".e2e");
const databaseFile = process.env.ANGUI_E2E_DATABASE_FILE ?? ".e2e/angui-e2e.db";
const databaseFiles = [databaseFile, `${databaseFile}-shm`, `${databaseFile}-wal`];
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const debugExecutable = (name) =>
  resolve(workspaceRoot, "target", "debug", `${name}${executableSuffix}`);
const frontendPort = process.env.ANGUI_E2E_FRONTEND_PORT ?? "5174";

function run(program, args, cwd = workspaceRoot) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(program, args, { cwd, env: process.env, stdio: "inherit" });
    child.once("error", rejectRun);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolveRun();
        return;
      }
      rejectRun(
        new Error(
          `${program} ${args.join(" ")} exited with ${code ?? signal ?? "an unknown error"}`,
        ),
      );
    });
  });
}

async function prepareDatabase() {
  await mkdir(e2eDirectory, { recursive: true });
  await Promise.all(
    databaseFiles.map((file) => rm(resolve(workspaceRoot, file), { force: true })),
  );
  await run("cargo", ["build", "--workspace", "--locked", "--bins"]);
  await run(debugExecutable("migration"), ["up"]);
  await run(debugExecutable("angui-admin"), ["bootstrap-demo"]);
}

function start(program, args, cwd) {
  return spawn(program, args, { cwd, env: process.env, stdio: "inherit" });
}

await prepareDatabase();

const backend = start(debugExecutable("angui"), [], workspaceRoot);
const frontend = start(
  process.execPath,
  [
    "node_modules/vite/bin/vite.js",
    "--host",
    "127.0.0.1",
    "--port",
    frontendPort,
    "--strictPort",
  ],
  frontendRoot,
);

let stopping = false;
// On Windows, inherited child stdio does not reliably keep this launcher
// referenced after Playwright's readiness probe. Keep the supervisor alive
// until Playwright explicitly terminates the web server.
const keepAlive = setInterval(() => {}, 60_000);
function stop(signal = "SIGTERM") {
  if (stopping) return;
  stopping = true;
  clearInterval(keepAlive);
  backend.kill(signal);
  frontend.kill(signal);
  setTimeout(() => {
    backend.kill("SIGKILL");
    frontend.kill("SIGKILL");
    // Child stdout handles can otherwise keep the launcher alive on Windows
    // after Playwright has finished. The children above are direct processes,
    // so this is only a bounded cleanup for this isolated E2E server pair.
    process.exit(process.exitCode ?? 0);
  }, 5_000).unref();
}

for (const signal of ["SIGINT", "SIGTERM"]) process.once(signal, () => stop(signal));

for (const [name, child] of [
  ["backend", backend],
  ["frontend", frontend],
]) {
  child.once("error", (error) => {
    console.error(`${name} failed to start:`, error);
    stop();
    process.exitCode = 1;
  });
  child.once("exit", (code, signal) => {
    if (!stopping) {
      console.error(`${name} exited unexpectedly with ${code ?? signal ?? "an unknown error"}`);
      stop();
      process.exitCode = 1;
    }
  });
}
