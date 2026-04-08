#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const {
  getSupportedPackageNames,
} = require("./platform.cjs");

const rootDir = path.join(__dirname, "..");
const nextVersion = process.argv[2];

if (!nextVersion) {
  console.error("Usage: node ./npm/prepare-release.cjs <next-version>");
  process.exit(1);
}

function updateJsonFile(filePath, mutator) {
  const absolutePath = path.join(rootDir, filePath);
  const parsed = JSON.parse(fs.readFileSync(absolutePath, "utf8"));
  const updated = mutator(parsed);
  fs.writeFileSync(absolutePath, `${JSON.stringify(updated, null, 2)}\n`);
}

function updateCargoToml(filePath, version) {
  const absolutePath = path.join(rootDir, filePath);
  const original = fs.readFileSync(absolutePath, "utf8");
  const updated = original.replace(
    /^version = "[^"]+"$/m,
    `version = "${version}"`,
  );

  if (updated === original) {
    if (original.includes(`version = "${version}"`)) {
      return;
    }

    throw new Error(`Could not update version in ${filePath}`);
  }

  fs.writeFileSync(absolutePath, updated);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readCargoPackageName(filePath) {
  const absolutePath = path.join(rootDir, filePath);
  const contents = fs.readFileSync(absolutePath, "utf8");
  let inPackageSection = false;

  for (const line of contents.split(/\r?\n/)) {
    const trimmed = line.trim();

    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inPackageSection = trimmed === "[package]";
      continue;
    }

    if (!inPackageSection) {
      continue;
    }

    const match = line.match(/^\s*name\s*=\s*"([^"]+)"/);
    if (match) {
      return match[1];
    }
  }

  throw new Error(`Could not read [package].name from ${filePath}`);
}

function updateCargoLock(filePath, packageName, version) {
  const absolutePath = path.join(rootDir, filePath);
  const original = fs.readFileSync(absolutePath, "utf8");
  const packageEntry = new RegExp(
    `(\\[\\[package\\]\\]\\r?\\nname = "${escapeRegExp(packageName)}"\\r?\\nversion = ")[^"]+"`,
  );
  const updated = original.replace(packageEntry, `$1${version}"`);

  if (updated === original) {
    const packageVersionEntry = new RegExp(
      `\\[\\[package\\]\\]\\r?\\nname = "${escapeRegExp(packageName)}"\\r?\\nversion = "${escapeRegExp(version)}"`,
    );
    if (packageVersionEntry.test(original)) {
      return;
    }

    throw new Error(`Could not update ${packageName} package version in ${filePath}`);
  }

  fs.writeFileSync(absolutePath, updated);
}

function updateRootPackageJson(pkg, version) {
  const optionalDependencies = Object.fromEntries(
    getSupportedPackageNames().map((packageName) => [packageName, version]),
  );

  return {
    ...pkg,
    version,
    optionalDependencies,
  };
}

const cargoPackageName = readCargoPackageName("Cargo.toml");

updateJsonFile("package.json", (pkg) =>
  updateRootPackageJson(pkg, nextVersion),
);
updateJsonFile("package-lock.json", (lockfile) => ({
  ...lockfile,
  name: "telegram-agent-cli",
  version: nextVersion,
  packages: {
    ...lockfile.packages,
    "": {
      ...lockfile.packages[""],
      name: "telegram-agent-cli",
      version: nextVersion,
      optionalDependencies: Object.fromEntries(
        getSupportedPackageNames().map((packageName) => [packageName, nextVersion]),
      ),
    },
    ...Object.fromEntries(
      getSupportedPackageNames().map((packageName) => [
        `node_modules/${packageName}`,
        {
          ...lockfile.packages?.[`node_modules/${packageName}`],
          version: nextVersion,
          optional: true,
        },
      ]),
    ),
  },
}));
for (const packageName of getSupportedPackageNames()) {
  updateJsonFile(`npm/packages/${packageName}/package.json`, (pkg) => ({
    ...pkg,
    version: nextVersion,
  }));
}
updateCargoToml("Cargo.toml", nextVersion);
updateCargoLock("Cargo.lock", cargoPackageName, nextVersion);
