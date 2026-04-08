import { chmodSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import {
  archiveFilenameForTarget,
  checksumFilenameForArchive,
  computeSha256,
  copyFile,
  ensureCleanDir,
  ensureDir,
  loadReleaseConfig,
  rootDir,
  runCommand,
} from "./release-helpers.mjs";

const require = createRequire(import.meta.url);
const { getTargetInfos } = require("../../npm/platform.cjs");

function parseArgs(argv) {
  const args = {};

  for (let index = 0; index < argv.length; index += 1) {
    const entry = argv[index];
    if (!entry.startsWith("--")) {
      continue;
    }

    const key = entry.slice(2);
    const nextValue = argv[index + 1];

    if (!nextValue || nextValue.startsWith("--")) {
      args[key] = true;
      continue;
    }

    args[key] = nextValue;
    index += 1;
  }

  return args;
}

function buildCommandForTarget(target) {
  if (target.includes("linux")) {
    return ["cargo", ["zigbuild", "--release", "--target", target]];
  }

  return ["cargo", ["build", "--release", "--target", target]];
}

function packageArchive(targetInfo, version, config, binaryPath, distDir) {
  const archiveFilename = archiveFilenameForTarget(
    config,
    version,
    targetInfo.target,
  );
  const archivePath = path.join(distDir, archiveFilename);

  if (targetInfo.archiveExtension === "zip") {
    runCommand("zip", ["-j", archivePath, binaryPath]);
  } else {
    runCommand("tar", [
      "-czf",
      archivePath,
      "-C",
      path.dirname(binaryPath),
      path.basename(binaryPath),
    ]);
  }

  const checksumFilename = checksumFilenameForArchive(archiveFilename);
  const checksumPath = path.join(distDir, checksumFilename);
  const checksum = computeSha256(archivePath);
  writeFileSync(checksumPath, `${checksum}  ${archiveFilename}\n`, "utf8");

  return {
    archiveFilename,
    archivePath,
    checksumFilename,
    checksumPath,
  };
}

function stageNpmPackage(targetInfo, binaryPath, distNpmDir) {
  const packageDir = path.join(distNpmDir, targetInfo.packageName);
  const stagedBinaryPath = path.join(
    packageDir,
    "bin",
    targetInfo.binaryFileName,
  );

  ensureDir(path.dirname(stagedBinaryPath));
  copyFile(
    path.join(
      rootDir,
      "npm",
      "packages",
      targetInfo.packageName,
      "package.json",
    ),
    path.join(packageDir, "package.json"),
  );
  copyFile(
    path.join(rootDir, "npm", "packages", "README.platform-package.md"),
    path.join(packageDir, "README.md"),
  );
  copyFile(binaryPath, stagedBinaryPath);

  if (!targetInfo.target.includes("windows")) {
    chmodSync(stagedBinaryPath, 0o755);
  }
}

const args = parseArgs(process.argv.slice(2));
const rootPackageJson = JSON.parse(
  readFileSync(path.join(rootDir, "package.json"), "utf8"),
);
const version = args.version ?? rootPackageJson.version;

if (!version) {
  throw new Error(
    "Usage: node ./scripts/release/build-release-artifacts.mjs --version <version>",
  );
}

if (rootPackageJson.version !== version) {
  throw new Error(
    `package.json version ${rootPackageJson.version} does not match requested ${version}. Run release:prepare first.`,
  );
}

const config = loadReleaseConfig();
const distDir = path.join(rootDir, "dist");
const distNpmDir = path.join(distDir, "npm");
const targetInfoByTarget = new Map(
  getTargetInfos().map((entry) => [entry.target, entry]),
);

ensureCleanDir(distDir);
ensureDir(distNpmDir);

for (const targetConfig of config.artifactTargets) {
  const targetInfo = targetInfoByTarget.get(targetConfig.target);
  if (!targetInfo) {
    throw new Error(`Unsupported platform target: ${targetConfig.target}.`);
  }

  const [command, commandArgs] = buildCommandForTarget(targetInfo.target);
  process.stdout.write(
    `[release-build] building ${targetInfo.target} via ${command} ${commandArgs.join(" ")}\n`,
  );
  runCommand(command, commandArgs);

  const binaryPath = path.join(
    rootDir,
    "target",
    targetInfo.target,
    "release",
    targetInfo.binaryFileName,
  );
  if (!existsSync(binaryPath)) {
    throw new Error(`Built binary not found: ${binaryPath}.`);
  }

  const archive = packageArchive(
    targetInfo,
    version,
    config,
    binaryPath,
    distDir,
  );
  stageNpmPackage(targetInfo, binaryPath, distNpmDir);

  process.stdout.write(
    `[release-build] staged ${targetInfo.packageName} and ${archive.archiveFilename}\n`,
  );
}
