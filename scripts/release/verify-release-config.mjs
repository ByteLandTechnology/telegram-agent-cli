import { appendFileSync, existsSync, readFileSync } from "node:fs";
import path from "node:path";
import {
  installScriptAbsolutePath,
  installScriptRelativePath,
  isPlaceholderRepository,
  isPlaceholderValue,
  loadReleaseConfig,
  prepareGeneratedSkillProject,
  releaseArtifactsDir,
  releaseAssetsDir,
  releaseEvidenceFilename,
  releaseEvidencePath,
  requiredArtifactTargets,
  resolveOwnerRepository,
  resolveSourceRepository,
  rootDir,
} from "./release-helpers.mjs";

const config = loadReleaseConfig();

function verifyPlaceholderReplaced(value, fieldPath) {
  if (!value) {
    throw new Error(`${fieldPath} is required.`);
  }

  if (isPlaceholderValue(value)) {
    throw new Error(
      `${fieldPath} still contains a REPLACE_WITH_* placeholder. Replace all placeholders in release/skill-release.config.json before running release automation.`,
    );
  }
}

function verifyGeneratedSkillConfig() {
  const { generatedSkill } = config;

  verifyPlaceholderReplaced(config.sourceSkillId, "sourceSkillId");
  verifyPlaceholderReplaced(
    generatedSkill.skillName,
    "generatedSkill.skillName",
  );
  verifyPlaceholderReplaced(
    generatedSkill.description,
    "generatedSkill.description",
  );
  verifyPlaceholderReplaced(generatedSkill.author, "generatedSkill.author");
  verifyPlaceholderReplaced(
    generatedSkill.projectPath,
    "generatedSkill.projectPath",
  );
  verifyPlaceholderReplaced(
    config.artifactBuild.binaryName,
    "artifactBuild.binaryName",
  );

  if (generatedSkill.skillName !== config.sourceSkillId) {
    throw new Error("generatedSkill.skillName must match sourceSkillId.");
  }

  if (config.artifactBuild.binaryName !== config.sourceSkillId) {
    throw new Error("artifactBuild.binaryName must match sourceSkillId.");
  }

  if (
    !generatedSkill.templates ||
    Object.keys(generatedSkill.templates).length === 0
  ) {
    throw new Error("generatedSkill.templates must not be empty.");
  }

  prepareGeneratedSkillProject(config);
}

function verifyGithubReleaseConfig() {
  if (!config.githubRelease) {
    throw new Error("githubRelease config is required.");
  }

  const ownerRepository = resolveOwnerRepository(config);
  const sourceRepository = resolveSourceRepository(config);

  if (ownerRepository !== sourceRepository) {
    throw new Error(
      `sourceRepository (${sourceRepository}) must match githubRelease owner repository (${ownerRepository}) for repo-native releases.`,
    );
  }

  verifyPlaceholderReplaced(
    config.githubRelease.installScriptPath,
    "githubRelease.installScriptPath",
  );
  verifyPlaceholderReplaced(
    config.githubRelease.releaseEvidenceFilename,
    "githubRelease.releaseEvidenceFilename",
  );

  const installScriptPath = installScriptAbsolutePath(config);
  if (!existsSync(installScriptPath)) {
    throw new Error(
      `Configured install helper is missing: ${installScriptRelativePath(config)}.`,
    );
  }

  const installScript = readFileSync(installScriptPath, "utf8");
  if (!installScript.includes("releases/tag/")) {
    throw new Error(
      `${installScriptRelativePath(config)} must resolve artifacts from the repository's tagged GitHub Releases.`,
    );
  }

  if (!Array.isArray(config.githubRelease.assetGlobPatterns)) {
    throw new Error("githubRelease.assetGlobPatterns must be an array.");
  }
}

function verifyArtifactTargets() {
  if (
    !Array.isArray(config.artifactTargets) ||
    config.artifactTargets.length === 0
  ) {
    throw new Error("artifactTargets must contain at least one entry.");
  }

  const requiredTargets = requiredArtifactTargets(config).map(
    (entry) => entry.target,
  );
  const minimumTargets = ["x86_64-unknown-linux-gnu", "aarch64-apple-darwin"];

  for (const minimumTarget of minimumTargets) {
    if (!requiredTargets.includes(minimumTarget)) {
      throw new Error(`Missing required artifact target: ${minimumTarget}.`);
    }
  }
}

function verifyGeneratedPackageBoundary() {
  if (!config.generatedPackageBoundary) {
    throw new Error("generatedPackageBoundary is required.");
  }

  const { packageLocalSupportExamples, repositoryOwnedAutomation } =
    config.generatedPackageBoundary;

  if (
    !Array.isArray(packageLocalSupportExamples) ||
    !packageLocalSupportExamples.length
  ) {
    throw new Error(
      "generatedPackageBoundary.packageLocalSupportExamples must contain at least one entry.",
    );
  }

  if (
    !Array.isArray(repositoryOwnedAutomation) ||
    !repositoryOwnedAutomation.length
  ) {
    throw new Error(
      "generatedPackageBoundary.repositoryOwnedAutomation must contain at least one entry.",
    );
  }

  const missingAutomationPaths = repositoryOwnedAutomation.filter(
    (entry) => !existsSync(path.join(rootDir, entry)),
  );

  if (missingAutomationPaths.length > 0) {
    throw new Error(
      `Configured repository-owned automation paths do not exist: ${missingAutomationPaths.join(", ")}.`,
    );
  }
}

function verifyOptionalSecondaryPublication() {
  const secondary = config.optionalSecondaryPublication;
  if (!secondary) {
    throw new Error("optionalSecondaryPublication config is required.");
  }

  if (secondary.enabled) {
    if (isPlaceholderRepository(secondary.destinationRepository)) {
      throw new Error(
        "optionalSecondaryPublication.destinationRepository must be configured when optionalSecondaryPublication.enabled is true.",
      );
    }
  }
}

verifyGeneratedSkillConfig();
verifyGithubReleaseConfig();
verifyArtifactTargets();
verifyGeneratedPackageBoundary();
verifyOptionalSecondaryPublication();

const ownerRepository = resolveOwnerRepository(config);

if (process.env.GITHUB_OUTPUT) {
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    [
      `github_release_assets_dir=${path.relative(rootDir, releaseAssetsDir(config)).replace(/\\/g, "/")}`,
      `install_script_path=${installScriptRelativePath(config)}`,
      `owner_repository=${ownerRepository}`,
      `release_artifacts_dir=${path.relative(rootDir, releaseArtifactsDir(config)).replace(/\\/g, "/")}`,
      `release_evidence_filename=${releaseEvidenceFilename(config)}`,
      `release_evidence_path=${path.relative(rootDir, releaseEvidencePath(config)).replace(/\\/g, "/")}`,
      `required_targets=${requiredArtifactTargets(config)
        .map((entry) => entry.target)
        .join(",")}`,
    ].join("\n") + "\n",
    "utf8",
  );
}

console.log(
  `Release configuration verified for repo-native publication in ${ownerRepository}.`,
);
