import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { resolve } from "node:path";

const workspaceRoot = resolve(import.meta.dirname, "../..");
const e2eDirectory = resolve(workspaceRoot, ".e2e");
const databaseFiles = ["angui-e2e.db", "angui-e2e.db-shm", "angui-e2e.db-wal"];
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const debugExecutable = (name) =>
  resolve(workspaceRoot, "target", "debug", `${name}${executableSuffix}`);

function run(program, args) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(program, args, {
      cwd: workspaceRoot,
      env: process.env,
      stdio: "inherit",
    });
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
    databaseFiles.map((file) => rm(resolve(e2eDirectory, file), { force: true })),
  );
  await run("cargo", ["build", "--workspace", "--locked", "--bins"]);
  await run(debugExecutable("migration"), ["up"]);
  await run(debugExecutable("angui-admin"), ["bootstrap-demo"]);
}

await prepareDatabase();

const server = spawn(debugExecutable("angui"), [], {
  cwd: workspaceRoot,
  env: process.env,
  stdio: "inherit",
});

let stopping = false;
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.once(signal, () => {
    if (stopping) return;
    stopping = true;
    server.kill(signal);
    setTimeout(() => server.kill("SIGKILL"), 10_000).unref();
  });
}

server.once("error", (error) => {
  console.error(error);
  process.exitCode = 1;
});
server.once("exit", (code, signal) => {
  process.exitCode = stopping ? 0 : (code ?? (signal ? 1 : 0));
});
