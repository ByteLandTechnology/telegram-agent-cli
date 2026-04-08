import { existsSync } from "node:fs";
import semanticRelease from "semantic-release";
import {
  loadReleaseConfig,
  releaseEvidencePath,
  resolveOwnerRepository,
  rootDir,
  runCommand,
  sourceReleaseUrl,
  writeJson,
} from "./release-helpers.mjs";

const dryRun = process.argv.includes("--dry-run");
const noCi = process.argv.includes("--no-ci");

const result = await semanticRelease(
  {
    ci: !noCi,
    dryRun,
  },
  {
    cwd: rootDir,
    env: process.env,
    stderr: process.stderr,
    stdout: process.stdout,
  },
);

const config = loadReleaseConfig();
const receiptPath = `${rootDir}/.work/release/last-publication-receipt.json`;

if (!existsSync(receiptPath)) {
  const noReleaseDetected = result === false;

  const ownerRepository = (() => {
    try {
      return resolveOwnerRepository(config);
    } catch {
      return (
        process.env[config.githubRelease.ownerRepositoryEnv] ||
        config.githubRelease.ownerRepository
      );
    }
  })();

  const receipt = {
    artifactResults: [],
    blockingReason:
      result !== false
        ? noReleaseDetected
          ? "semantic-release found no releasable changes"
          : "semantic-release completed without generating a release receipt"
        : "semantic-release found no releasable changes",
    githubReleaseUrl: null,
    installScriptPath: config.githubRelease.installScriptPath,
    optionalSecondaryPublicationEnabled: Boolean(
      config.optionalSecondaryPublication?.enabled,
    ),
    publicationMode: dryRun
      ? "dry_run"
      : process.env.GITHUB_ACTIONS === "true"
        ? "live_release"
        : "report_only",
    publicationResult: "skipped",
    publishRoot: ".work/release/github-release",
    publishedAt: new Date().toISOString(),
    releaseEvidencePath: existsSync(releaseEvidencePath(config))
      ? ".work/release/github-release/release-evidence.json"
      : null,
    runResult:
      result !== false
        ? noReleaseDetected
          ? "no_release"
          : "prepared"
        : "no_release",
    sourceCommitSha: runCommand("git", ["rev-parse", "HEAD"], {
      cwd: rootDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim(),
    sourceGitTag: null,
    sourceRepository: ownerRepository,
    sourceSkillId: config.sourceSkillId,
    sourceVersion: null,
  };

  if (receipt.sourceGitTag) {
    receipt.githubReleaseUrl = sourceReleaseUrl(
      ownerRepository,
      receipt.sourceGitTag,
    );
  }

  writeJson(receiptPath, receipt);
}
