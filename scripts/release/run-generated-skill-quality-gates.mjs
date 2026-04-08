import { chmodSync, existsSync, readFileSync } from "node:fs";
import path from "node:path";
import {
  installScriptRelativePath,
  loadReleaseConfig,
  prepareGeneratedSkillProject,
  releaseEvidenceFilename,
  runCommand,
} from "./release-helpers.mjs";

const config = loadReleaseConfig();
const projectDir = prepareGeneratedSkillProject(config);

const generatedReadme = readFileSync(
  path.join(projectDir, "README.md"),
  "utf8",
);
const generatedSkill = readFileSync(path.join(projectDir, "SKILL.md"), "utf8");
const installScriptPath = path.join(
  projectDir,
  installScriptRelativePath(config),
);

if (!existsSync(installScriptPath)) {
  throw new Error(
    `Generated project is missing install helper ${installScriptRelativePath(config)}.`,
  );
}

chmodSync(installScriptPath, 0o755);

if (!generatedReadme.includes("scripts/install-current-release.sh")) {
  throw new Error(
    "Generated README.md must document scripts/install-current-release.sh.",
  );
}

if (!generatedSkill.includes("GitHub Release")) {
  throw new Error(
    "Generated SKILL.md must mention repo-native GitHub Release installation.",
  );
}

if (!generatedReadme.includes(releaseEvidenceFilename(config))) {
  throw new Error(
    "Generated README.md must mention the release evidence file.",
  );
}

runCommand("cargo", ["fmt"], { cwd: projectDir });
runCommand("cargo", ["fmt", "--check"], { cwd: projectDir });
runCommand("cargo", ["clippy", "--", "-D", "warnings"], { cwd: projectDir });
runCommand("cargo", ["test"], { cwd: projectDir });

console.log(
  `Target-project quality gates passed for ${config.generatedSkill.skillName} in ${projectDir}.`,
);
