import { chmodSync, writeFileSync } from "node:fs";
import {
  buildBinaryFromProjectPath,
  getArtifactTarget,
  loadReleaseConfig,
  prepareGeneratedSkillProject,
  relativeToRoot,
  releaseBuildBinaryPath,
  runCommand,
  targetArtifactsDir,
  writeJson,
} from "./release-helpers.mjs";

const [target] = process.argv.slice(2);
const synthetic = process.argv.includes("--synthetic");

if (!target) {
  throw new Error(
    "Usage: node scripts/release/build-cli-artifact.mjs <target> [--synthetic]",
  );
}

const config = loadReleaseConfig();
const targetConfig = getArtifactTarget(config, target);
const projectDir = prepareGeneratedSkillProject(config);
const outputDir = targetArtifactsDir(config, target);
const binaryPath = releaseBuildBinaryPath(config, target);

runCommand("mkdir", ["-p", outputDir]);
runCommand("mkdir", ["-p", `${outputDir}/binary`]);

if (synthetic) {
  const stubContents = [
    "#!/usr/bin/env sh",
    `echo \"${config.generatedSkill.skillName} synthetic rehearsal artifact for ${target}\"`,
    "",
  ].join("\n");

  writeFileSync(binaryPath, stubContents, "utf8");
  chmodSync(binaryPath, 0o755);
} else {
  runCommand("cargo", ["build", "--release", "--target", target], {
    cwd: projectDir,
  });
  runCommand("cp", [buildBinaryFromProjectPath(config, target), binaryPath]);
}

const metadata = {
  archiveBasenamePrefix: `${config.sourceSkillId}-<version>-${target}`,
  artifactOrigin: synthetic ? "synthetic_rehearsal" : "cargo_build",
  binaryName: config.artifactBuild.binaryName,
  binaryPath: relativeToRoot(binaryPath),
  builtAt: new Date().toISOString(),
  generatedSkillProjectPath: relativeToRoot(projectDir),
  releaseSurface: "github_release_asset",
  required: targetConfig.required !== false,
  runner: targetConfig.runner,
  target,
};

writeJson(`${outputDir}/build-metadata.json`, metadata);
process.stdout.write(`${JSON.stringify(metadata)}\n`);
