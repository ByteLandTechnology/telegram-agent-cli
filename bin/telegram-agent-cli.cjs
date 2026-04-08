#!/usr/bin/env node

const { spawn } = require("node:child_process");
const { ensureBinary } = require("../npm/install.cjs");

async function main() {
  try {
    const binaryPath = await ensureBinary();
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: "inherit",
      env: process.env,
    });

    child.on("error", (error) => {
      console.error(
        `[telegram-agent-cli] Failed to start binary: ${error.message}`,
      );
      process.exit(1);
    });

    child.on("exit", (code, signal) => {
      if (signal) {
        process.kill(process.pid, signal);
        return;
      }

      process.exit(code ?? 1);
    });
  } catch (error) {
    console.error(`[telegram-agent-cli] ${error.message}`);
    process.exit(1);
  }
}

void main();
