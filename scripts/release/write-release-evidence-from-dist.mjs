import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import {
  archiveFilenameForTarget,
  computeSha256,
  loadReleaseConfig,
  relativeToRoot,
  releaseEvidencePath,
  resolveOwnerRepository,
  resolveSourceRepository,
  rootDir,
  runCommand,
  sourceReleaseUrl,
  writeJson,
} from "./release-helpers.mjs";

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

function normalizeGitTag(value) {
  if (!value) {
    return null;
  }

  return value.replace(/^refs\/tags\//, "");
}

function parseChecksum(filePath) {
  const contents = readFileSync(filePath, "utf8").trim();
  const match = contents.match(/^([a-f0-9]{64})\s+\*?(.+)$/i);
  if (!match) {
    throw new Error(`Invalid checksum file format: ${filePath}.`);
  }

  return {
    archiveFilename: path.basename(match[2]),
    sha256: match[1].toLowerCase(),
  };
}

function summarizeNpmPackages(distDir, version) {
  const npmRoot = path.join(distDir, "npm");
  if (!existsSync(npmRoot) || !statSync(npmRoot).isDirectory()) {
    return null;
  }

  const platformPackages = readdirSync(npmRoot)
    .map((entry) => path.join(npmRoot, entry))
    .filter((entryPath) => statSync(entryPath).isDirectory())
    .map((packageDir) => {
      const manifestPath = path.join(packageDir, "package.json");
      if (!existsSync(manifestPath)) {
        throw new Error(`Missing staged npm package manifest: ${manifestPath}.`);
      }

      const packageJson = JSON.parse(readFileSync(manifestPath, "utf8"));
      const binaryEntries = Object.values(packageJson.bin ?? {});
      const bundledBinaryPath =
        binaryEntries.length > 0
          ? path.join(packageDir, binaryEntries[0])
          : null;

      return {
        binaryPath: bundledBinaryPath ? relativeToRoot(bundledBinaryPath) : null,
        name: packageJson.name,
        packageRoot: relativeToRoot(packageDir),
        version: packageJson.version,
      };
    })
    .sort((left, right) => left.name.localeCompare(right.name));

  const rootPackageJson = JSON.parse(
    readFileSync(path.join(rootDir, "package.json"), "utf8"),
  );

  if (rootPackageJson.version !== version) {
    throw new Error(
      `Root package version ${rootPackageJson.version} does not match ${version}.`,
    );
  }

  return {
    coordinatingPackage: {
      name: rootPackageJson.name,
      version: rootPackageJson.version,
    },
    platformPackages,
  };
}

const args = parseArgs(process.argv.slice(2));
const version = args.version ?? process.env.VERSION;
const gitTag = normalizeGitTag(
  args["git-tag"] ?? process.env.GITHUB_REF_NAME ?? process.env.GITHUB_REF,
);

if (!version || !gitTag) {
  throw new Error(
    "Usage: node scripts/release/write-release-evidence-from-dist.mjs --version <version> --git-tag <tag> [--dist <dir>]",
  );
}

const config = loadReleaseConfig();
const distDir = path.resolve(rootDir, args.dist ?? "./dist");
const evidencePath = releaseEvidencePath(config);
const manifestPath = path.join(rootDir, config.metadataFilename);
const ownerRepository = resolveOwnerRepository(config);
const sourceRepository = resolveSourceRepository(config);
const publishedAt = new Date().toISOString();
const sourceCommitSha = runCommand("git", ["rev-parse", "HEAD"], {
  cwd: rootDir,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "pipe"],
}).trim();

if (!existsSync(distDir) || !statSync(distDir).isDirectory()) {
  throw new Error(`dist directory does not exist: ${distDir}.`);
}

const artifactResults = config.artifactTargets.map((targetConfig) => {
  const archiveFilename = archiveFilenameForTarget(
    config,
    version,
    targetConfig.target,
  );
  const archivePath = path.join(distDir, archiveFilename);
  const checksumPath = path.join(distDir, `${archiveFilename}.sha256`);

  if (!existsSync(archivePath)) {
    if (targetConfig.required === false) {
      return null;
    }

    throw new Error(`Missing release archive: ${archivePath}.`);
  }

  if (!existsSync(checksumPath)) {
    throw new Error(`Missing release checksum: ${checksumPath}.`);
  }

  const checksumRecord = parseChecksum(checksumPath);
  if (checksumRecord.archiveFilename !== archiveFilename) {
    throw new Error(
      `Checksum file ${checksumPath} does not point to ${archiveFilename}.`,
    );
  }

  const computedSha = computeSha256(archivePath);
  if (checksumRecord.sha256 !== computedSha) {
    throw new Error(
      `Checksum mismatch for ${archiveFilename}: expected ${checksumRecord.sha256}, computed ${computedSha}.`,
    );
  }

  return {
    archiveFilename,
    archivePath: relativeToRoot(archivePath),
    checksumFilename: `${archiveFilename}.sha256`,
    checksumPath: relativeToRoot(checksumPath),
    required: targetConfig.required !== false,
    runner: targetConfig.runner,
    sha256: computedSha,
    targetVariant: targetConfig.target,
  };
}).filter(Boolean);

const releaseUrl = sourceReleaseUrl(ownerRepository, gitTag);
const npmPackageSet = summarizeNpmPackages(distDir, version);
const releaseEvidence = {
  artifactResults,
  generatedPackageBoundary: config.generatedPackageBoundary,
  githubRelease: {
    assetDirectory: relativeToRoot(distDir),
    installCommandExample: config.githubRelease.installCommandExample,
    installScriptPath: config.githubRelease.installScriptPath,
    ownerRepository,
    releaseEvidenceFilename: path.basename(evidencePath),
    releaseUrl,
  },
  metadataVersion: 1,
  npmPackageSet,
  publicationMode: process.env.GITHUB_ACTIONS === "true" ? "live_release" : "dry_run",
  publishedAt,
  sourceCommitSha,
  sourceGitTag: gitTag,
  sourceRepository,
  sourceSkillId: config.sourceSkillId,
  sourceVersion: version,
};

const manifest = {
  artifactResults,
  githubReleaseAssets: [
    "dist/*.tar.gz",
    "dist/*.zip",
    "dist/*.sha256",
    relativeToRoot(evidencePath),
    relativeToRoot(manifestPath),
  ],
  npmPackageSet,
  publicationMode: releaseEvidence.publicationMode,
  publishedAt,
  releaseEvidencePath: relativeToRoot(evidencePath),
  releaseUrl,
  sourceCommitSha,
  sourceGitTag: gitTag,
  sourceRepository,
  sourceSkillId: config.sourceSkillId,
  sourceVersion: version,
};

writeJson(evidencePath, releaseEvidence);
writeJson(manifestPath, manifest);
process.stdout.write(
  `${JSON.stringify({ evidencePath: relativeToRoot(evidencePath), manifestPath: relativeToRoot(manifestPath) })}\n`,
);
