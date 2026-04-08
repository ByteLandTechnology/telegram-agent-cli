#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const { bestEffortInstallBinary } = require("./install.cjs");

if (process.env.TELEGRAM_CLI_SKIP_POSTINSTALL_DOWNLOAD === "1") {
  process.exit(0);
}

if (fs.existsSync(path.join(__dirname, "..", ".git"))) {
  process.exit(0);
}

void bestEffortInstallBinary();
