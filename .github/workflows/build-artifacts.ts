#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const target = process.env.TARGET;
const version = process.env.VERSION;

if (!target || !version) {
  console.error("Missing TARGET or VERSION environment variable");
  process.exit(1);
}

const binaryName = "telegram-agent-cli";
const artifactsRoot = ".work/artifacts";
const binaryFileName = target.includes("windows")
  ? `${binaryName}.exe`
  : binaryName;
const archiveFormat = target.includes("windows") ? "zip" : "tar.gz";
const archiveName = `${binaryName}-${version}-${target}.${archiveFormat}`;

mkdirSync(artifactsRoot, { recursive: true });

const builtBinary = `target/${target}/release/${binaryFileName}`;
if (!existsSync(builtBinary)) {
  console.error(`Binary not found: ${builtBinary}`);
  process.exit(1);
}

if (archiveFormat === "zip") {
  if (process.platform === "win32") {
    execFileSync("powershell", [
      "-NoProfile",
      "-Command",
      `Compress-Archive -Path '${builtBinary}' -DestinationPath '${process.cwd()}/${artifactsRoot}/${archiveName}' -Force`,
    ]);
  } else {
    execFileSync("zip", ["-j", `${artifactsRoot}/${archiveName}`, builtBinary]);
  }
} else {
  execFileSync("tar", [
    "-czf",
    `${artifactsRoot}/${archiveName}`,
    "-C",
    `target/${target}/release`,
    binaryFileName,
  ]);
}

const archivePath = `${artifactsRoot}/${archiveName}`;
const sha256 = createHash("sha256")
  .update(readFileSync(archivePath))
  .digest("hex");
writeFileSync(`${archivePath}.sha256`, `${sha256}  ${archiveName}\n`);

console.log(`Built: ${archiveName}`);
