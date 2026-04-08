import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

export function normalizePath(value) {
  return value.replace(/\\/g, "/");
}

export function relativeToRoot(value) {
  return normalizePath(path.relative(rootDir, value));
}

export function ensureDir(directoryPath) {
  mkdirSync(directoryPath, { recursive: true });
}

export function ensureCleanDir(directoryPath) {
  rmSync(directoryPath, { recursive: true, force: true });
  ensureDir(directoryPath);
}

export function assertDirectory(directoryPath, description) {
  if (!existsSync(directoryPath)) {
    throw new Error(`${description} does not exist: ${directoryPath}.`);
  }

  if (!statSync(directoryPath).isDirectory()) {
    throw new Error(`${description} is not a directory: ${directoryPath}.`);
  }
}

export function loadReleaseConfig() {
  return JSON.parse(
    readFileSync(
      path.join(rootDir, "release/skill-release.config.json"),
      "utf8",
    ),
  );
}

export function readJson(filePath, fallbackValue = null) {
  if (!existsSync(filePath)) {
    return fallbackValue;
  }

  return JSON.parse(readFileSync(filePath, "utf8"));
}

export function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

export function runCommand(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: options.cwd ?? rootDir,
    env: {
      ...process.env,
      ...options.env,
    },
    stdio: options.stdio ?? "inherit",
    encoding: options.encoding,
  });
}

export function computeSha256(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

export function copyDirectoryContents(sourceDir, destinationDir, options = {}) {
  ensureDir(destinationDir);

  for (const entry of readdirSync(sourceDir)) {
    if (options.exclude?.includes(entry)) {
      continue;
    }

    cpSync(path.join(sourceDir, entry), path.join(destinationDir, entry), {
      recursive: true,
      force: true,
    });
  }
}

export function copyFile(sourcePath, destinationPath) {
  ensureDir(path.dirname(destinationPath));
  copyFileSync(sourcePath, destinationPath);
}

export function isPlaceholderValue(value) {
  return typeof value === "string" && value.includes("REPLACE_WITH_");
}

export function isPlaceholderRepository(value) {
  return !value || value.includes("REPLACE_WITH_OWNER/REPO");
}

export function releaseArtifactsDir(config) {
  return path.join(rootDir, config.artifactBuild.artifactsDir);
}

export function targetArtifactsDir(config, target) {
  return path.join(releaseArtifactsDir(config), target);
}

export function targetBuildMetadataPath(config, target) {
  return path.join(targetArtifactsDir(config, target), "build-metadata.json");
}

export function releaseBuildBinaryPath(config, target) {
  const extension = target.includes("windows") ? ".exe" : "";
  return path.join(
    targetArtifactsDir(config, target),
    "binary",
    `${config.artifactBuild.binaryName}${extension}`,
  );
}

export function buildBinaryFromProjectPath(config, target) {
  const extension = target.includes("windows") ? ".exe" : "";
  return path.join(
    rootDir,
    config.generatedSkill.projectPath,
    "target",
    target,
    "release",
    `${config.artifactBuild.binaryName}${extension}`,
  );
}

export function getArtifactTarget(config, target) {
  const match = config.artifactTargets.find((entry) => entry.target === target);

  if (!match) {
    throw new Error(`Unknown artifact target: ${target}.`);
  }

  return match;
}

export function requiredArtifactTargets(config) {
  return config.artifactTargets.filter((entry) => entry.required);
}

export function archiveFilenameForTarget(config, version, target) {
  const targetConfig = getArtifactTarget(config, target);
  const archiveFormat = targetConfig.archiveFormat || "tar.gz";
  return `${config.sourceSkillId}-${version}-${target}.${archiveFormat}`;
}

export function checksumFilenameForArchive(archiveFilename) {
  return `${archiveFilename}.sha256`;
}

export function releaseAssetsDir(config) {
  return path.join(rootDir, config.githubRelease.releaseAssetsDir);
}

export function releaseEvidenceFilename(config) {
  return config.githubRelease.releaseEvidenceFilename;
}

export function releaseEvidencePath(config) {
  return path.join(releaseAssetsDir(config), releaseEvidenceFilename(config));
}

export function installScriptRelativePath(config) {
  return config.githubRelease.installScriptPath;
}

export function installScriptAbsolutePath(config) {
  return path.join(rootDir, installScriptRelativePath(config));
}

export function resolveOwnerRepository(config) {
  const resolved =
    process.env[config.githubRelease.ownerRepositoryEnv] ||
    config.githubRelease.ownerRepository;

  if (isPlaceholderRepository(resolved)) {
    throw new Error(
      [
        "Missing source repository identity.",
        `Set ${config.githubRelease.ownerRepositoryEnv} or update release/skill-release.config.json.`,
      ].join(" "),
    );
  }

  return resolved;
}

export function resolveSourceRepository(config) {
  const configured =
    process.env[config.sourceRepositoryEnv || "GITHUB_REPOSITORY"] ||
    config.sourceRepository;

  if (!configured || isPlaceholderRepository(configured)) {
    return resolveOwnerRepository(config);
  }

  return configured;
}

export function sourceReleaseUrl(ownerRepository, gitTag) {
  return `https://github.com/${ownerRepository}/releases/tag/${gitTag}`;
}

export function detectPublicationMode(config) {
  if (process.env.SKILL_RELEASE_PUBLICATION_MODE) {
    return process.env.SKILL_RELEASE_PUBLICATION_MODE;
  }

  if (process.env.GITHUB_ACTIONS === "true") {
    return "live_release";
  }

  return "dry_run";
}

function skillNameToSnake(skillName) {
  return skillName.replace(/-/g, "_");
}

function skillNameToPascal(skillName) {
  return skillName
    .split("-")
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join("");
}

function templateTokens(config) {
  const { author, description, rustEdition, skillName, version } =
    config.generatedSkill;

  return {
    AUTHOR: author ?? "",
    CURRENT_DATE: new Date().toISOString().slice(0, 10),
    DESCRIPTION: description,
    RUST_EDITION: rustEdition,
    SKILL_NAME: skillName,
    SKILL_NAME_PASCAL: skillNameToPascal(skillName),
    SKILL_NAME_SNAKE: skillNameToSnake(skillName),
    SKILL_NAME_UPPER: skillName.toUpperCase(),
    VERSION: version,
  };
}

function renderTemplate(templatePath, tokens) {
  let content = readFileSync(templatePath, "utf8");

  for (const [tokenName, tokenValue] of Object.entries(tokens)) {
    content = content.replaceAll(`{{${tokenName}}}`, tokenValue);
  }

  if (content.includes("{{") || content.includes("}}")) {
    throw new Error(
      `Unexpanded template token detected in ${relativeToRoot(templatePath)}.`,
    );
  }

  return content;
}

export function prepareGeneratedSkillProject(config) {
  const projectDir = path.join(rootDir, config.generatedSkill.projectPath);
  const tokens = templateTokens(config);

  ensureCleanDir(projectDir);
  ensureDir(path.join(projectDir, "src"));
  ensureDir(path.join(projectDir, "tests"));

  for (const [templateRelativePath, outputRelativePath] of Object.entries(
    config.generatedSkill.templates,
  )) {
    const templatePath = path.join(rootDir, templateRelativePath);
    if (!existsSync(templatePath)) {
      throw new Error(`Missing release template: ${templateRelativePath}.`);
    }

    let rendered = renderTemplate(templatePath, tokens);

    if (!tokens.AUTHOR && outputRelativePath === "Cargo.toml") {
      rendered = rendered.replace(/^authors = \[""\]\n/m, "");
    }

    if (!tokens.AUTHOR && outputRelativePath === "README.md") {
      rendered = rendered.replace(/\n## Author\n\n[\s\S]*?\n---\n/m, "\n---\n");
    }

    const outputPath = path.join(projectDir, outputRelativePath);
    ensureDir(path.dirname(outputPath));
    writeFileSync(outputPath, rendered, "utf8");

    if (outputRelativePath.endsWith(".sh")) {
      chmodSync(outputPath, 0o755);
    }
  }

  return projectDir;
}
