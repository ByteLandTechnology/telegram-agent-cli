import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import {
  loadReleaseConfig,
  releaseEvidencePath,
  rootDir,
} from "./release-helpers.mjs";

const require = createRequire(import.meta.url);
const { getSupportedPackageNames } = require("../../npm/platform.cjs");

const rootPackageJsonPath = path.join(rootDir, "package.json");

export function parseArgs(argv) {
  const args = {
    mode: "dry-run",
  };

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

export function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function normalizeGitTag(value) {
  if (!value) {
    return null;
  }

  return value.replace(/^refs\/tags\//, "");
}

function summarizeReleaseEvidence(evidencePath, expectedVersion, expectedGitTag) {
  const resolvedPath = evidencePath
    ? path.resolve(rootDir, evidencePath)
    : evidencePath;

  if (!resolvedPath) {
    return {
      checked: false,
      exists: false,
      path: null,
    };
  }

  if (!existsSync(resolvedPath)) {
    return {
      checked: false,
      exists: false,
      path: resolvedPath,
    };
  }

  const evidence = readJson(resolvedPath);
  const errors = [];

  if (evidence.sourceVersion !== expectedVersion) {
    errors.push(
      `release evidence version ${evidence.sourceVersion ?? "unknown"} does not match ${expectedVersion}.`,
    );
  }

  if (
    expectedGitTag &&
    evidence.sourceGitTag &&
    normalizeGitTag(evidence.sourceGitTag) !== expectedGitTag
  ) {
    errors.push(
      `release evidence git tag ${evidence.sourceGitTag} does not match ${expectedGitTag}.`,
    );
  }

  return {
    checked: true,
    errors,
    exists: true,
    path: resolvedPath,
    sourceGitTag: evidence.sourceGitTag ?? null,
    sourceVersion: evidence.sourceVersion ?? null,
  };
}

function rootOptionalDependencies(rootPackageJson) {
  return rootPackageJson.optionalDependencies ?? {};
}

function binaryPathForPackage(packageDirectory, packageJson) {
  const binEntries = Object.values(packageJson.bin ?? {});
  if (binEntries.length === 0) {
    return null;
  }

  return path.join(packageDirectory, binEntries[0]);
}

export function expectedReleaseEvidencePath() {
  try {
    return releaseEvidencePath(loadReleaseConfig());
  } catch {
    return null;
  }
}

export function verifyPackageSet(options = {}) {
  const rootPackageJson = readJson(rootPackageJsonPath);
  const expectedVersion = options.version ?? rootPackageJson.version;
  const expectedGitTag = normalizeGitTag(
    options.gitTag ?? process.env.GITHUB_REF_NAME ?? process.env.GITHUB_REF,
  ) ?? `v${expectedVersion}`;
  const packageNames = getSupportedPackageNames();
  const errors = [];
  const warnings = [];

  if (rootPackageJson.version !== expectedVersion) {
    errors.push(
      `root package version ${rootPackageJson.version} does not match ${expectedVersion}.`,
    );
  }

  if (expectedGitTag !== `v${expectedVersion}`) {
    errors.push(
      `git tag ${expectedGitTag} does not match the expected v${expectedVersion} tag.`,
    );
  }

  const optionalDependencyEntries = rootOptionalDependencies(rootPackageJson);
  const optionalDependencyNames = Object.keys(optionalDependencyEntries).sort();
  const expectedDependencyNames = [...packageNames].sort();

  if (
    JSON.stringify(optionalDependencyNames) !==
    JSON.stringify(expectedDependencyNames)
  ) {
    errors.push(
      `root optionalDependencies must match the supported platform package set: ${expectedDependencyNames.join(", ")}.`,
    );
  }

  for (const packageName of expectedDependencyNames) {
    if (optionalDependencyEntries[packageName] !== expectedVersion) {
      errors.push(
        `root optional dependency ${packageName} must point to ${expectedVersion}.`,
      );
    }
  }

  const sourcePackages = packageNames.map((packageName) => {
    const manifestPath = path.join(
      rootDir,
      "npm",
      "packages",
      packageName,
      "package.json",
    );
    const packageJson = readJson(manifestPath);

    if (packageJson.name !== packageName) {
      errors.push(
        `source package manifest ${manifestPath} has name ${packageJson.name}; expected ${packageName}.`,
      );
    }

    if (packageJson.version !== expectedVersion) {
      errors.push(
        `source package ${packageName} version ${packageJson.version} does not match ${expectedVersion}.`,
      );
    }

    return {
      manifestPath,
      name: packageName,
      packageJson,
    };
  });

  const stagedRootOption = options.stagedRoot ?? options.stagedDist;
  const stagedRoot = stagedRootOption
    ? path.resolve(rootDir, stagedRootOption)
    : null;
  const stagedPackages = [];

  if (stagedRoot) {
    if (!existsSync(stagedRoot) || !statSync(stagedRoot).isDirectory()) {
      errors.push(`staged npm package root does not exist: ${stagedRoot}.`);
    } else {
      for (const packageName of packageNames) {
        const packageDir = path.join(stagedRoot, packageName);
        const manifestPath = path.join(packageDir, "package.json");

        if (!existsSync(packageDir) || !statSync(packageDir).isDirectory()) {
          errors.push(`staged package directory is missing: ${packageDir}.`);
          continue;
        }

        if (!existsSync(manifestPath)) {
          errors.push(`staged package manifest is missing: ${manifestPath}.`);
          continue;
        }

        const packageJson = readJson(manifestPath);
        if (packageJson.name !== packageName) {
          errors.push(
            `staged package manifest ${manifestPath} has name ${packageJson.name}; expected ${packageName}.`,
          );
        }

        if (packageJson.version !== expectedVersion) {
          errors.push(
            `staged package ${packageName} version ${packageJson.version} does not match ${expectedVersion}.`,
          );
        }

        const binaryPath = binaryPathForPackage(packageDir, packageJson);
        if (!binaryPath || !existsSync(binaryPath)) {
          errors.push(
            `staged package ${packageName} is missing its bundled binary.`,
          );
        }

        const readmePath = path.join(packageDir, "README.md");
        if (!existsSync(readmePath)) {
          warnings.push(`staged package ${packageName} is missing README.md.`);
        }

        stagedPackages.push({
          binaryPath,
          manifestPath,
          name: packageName,
          packageDir,
          packageJson,
          readmePath,
        });
      }
    }
  }

  const evidencePath =
    options.releaseEvidencePath ?? expectedReleaseEvidencePath();
  const releaseEvidence = summarizeReleaseEvidence(
    evidencePath,
    expectedVersion,
    expectedGitTag,
  );

  if (releaseEvidence.exists && releaseEvidence.checked) {
    errors.push(...releaseEvidence.errors);
  } else if (options.requireReleaseEvidence) {
    errors.push(
      `release evidence is required but was not found at ${releaseEvidence.path ?? "the configured release evidence path"}.`,
    );
  } else if (options.releaseEvidencePath && !releaseEvidence.exists) {
    warnings.push(`release evidence not found at ${releaseEvidence.path}.`);
  }

  return {
    expectedGitTag,
    expectedVersion,
    packageNames,
    releaseEvidence,
    rootPackage: {
      manifestPath: rootPackageJsonPath,
      packageJson: rootPackageJson,
    },
    sourcePackages,
    stagedPackages,
    stagedRoot,
    validation: {
      errors,
      ok: errors.length === 0,
      warnings,
    },
  };
}

export function formatVerificationSummary(report) {
  const lines = [
    `Expected version: ${report.expectedVersion}`,
    `Expected git tag: ${report.expectedGitTag}`,
    `Root package: ${report.rootPackage.packageJson.name}@${report.rootPackage.packageJson.version}`,
    `Platform packages: ${report.packageNames.join(", ")}`,
  ];

  if (report.stagedRoot) {
    lines.push(`Staged package root: ${report.stagedRoot}`);
  }

  if (report.releaseEvidence.exists) {
    lines.push(`Release evidence: ${report.releaseEvidence.path}`);
  } else {
    lines.push("Release evidence: not present");
  }

  if (report.validation.warnings.length > 0) {
    lines.push(`Warnings: ${report.validation.warnings.length}`);
  }

  if (report.validation.ok) {
    lines.push("Verification status: ok");
  } else {
    lines.push("Verification status: failed");
    for (const error of report.validation.errors) {
      lines.push(`- ${error}`);
    }
  }

  return lines.join("\n");
}
