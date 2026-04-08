import { appendFileSync, existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const receiptPath = path.join(
  rootDir,
  ".work/release/last-publication-receipt.json",
);
const changelogPath = path.join(rootDir, "CHANGELOG.md");

if (!existsSync(receiptPath)) {
  throw new Error(
    "Missing release receipt. Run semantic-release before preparing the optional secondary publication message.",
  );
}

if (!existsSync(changelogPath)) {
  throw new Error("Missing CHANGELOG.md.");
}

const receipt = JSON.parse(readFileSync(receiptPath, "utf8"));
if (!receipt.sourceVersion) {
  throw new Error("Release receipt is missing sourceVersion.");
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function extractChangelogBody(changelog, version) {
  const headingPattern = new RegExp(
    `^## .*\\b${escapeRegex(version)}\\b.*$`,
    "m",
  );
  const headingMatch = changelog.match(headingPattern);

  if (!headingMatch || headingMatch.index === undefined) {
    throw new Error(`Could not find changelog section for version ${version}.`);
  }

  const afterHeading = changelog
    .slice(headingMatch.index + headingMatch[0].length)
    .replace(/^\r?\n+/, "");
  const nextHeadingIndex = afterHeading.search(/^#{1,2} /m);
  const rawBody =
    nextHeadingIndex === -1
      ? afterHeading
      : afterHeading.slice(0, nextHeadingIndex);
  const body = rawBody.trim();

  if (!body) {
    throw new Error(`Changelog section for version ${version} was empty.`);
  }

  return body;
}

const changelog = readFileSync(changelogPath, "utf8");
const body = extractChangelogBody(changelog, receipt.sourceVersion);
const fullCommitMessage = [
  `chore(release): mirror ${receipt.sourceSkillId} ${receipt.sourceVersion}`,
  "",
  body,
  "",
  "Optional secondary publication evidence:",
  `- release url: ${receipt.githubReleaseUrl || "not available"}`,
  `- install helper: ${receipt.installScriptPath || "not available"}`,
  `- release evidence: ${receipt.releaseEvidencePath || "not available"}`,
].join("\n");

if (process.env.GITHUB_OUTPUT) {
  const delimiter = "__CODEX_FULL_COMMIT_MESSAGE__";
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `full_commit_message<<${delimiter}\n${fullCommitMessage}\n${delimiter}\n`,
    "utf8",
  );
}

process.stdout.write(
  `Prepared optional secondary publication commit message for ${receipt.sourceSkillId} ${receipt.sourceVersion}.\n`,
);
