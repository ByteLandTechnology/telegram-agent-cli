const fs = require("node:fs");
const path = require("node:path");

const {
  getPlatformInfo,
  getSupportedPlatforms,
} = require("./platform.cjs");

function formatUnsupportedPlatformError() {
  return [
    `Unsupported platform: ${process.platform}-${process.arch}.`,
    `Supported npm targets: ${getSupportedPlatforms().join(", ")}.`,
    "Build from source with Cargo instead.",
  ].join(" ");
}

function formatMissingPackageError(info, resolveFrom) {
  return [
    `The optional platform package ${info.packageName}@${info.version} is not installed.`,
    `Expected host target: ${info.platform}-${info.arch}.`,
    `Lookup root: ${resolveFrom}.`,
    `Reinstall telegram-agent-cli with optional dependencies enabled, or install ${info.packageName}@${info.version} explicitly.`,
  ].join(" ");
}

function resolvePlatformPackageJson(info, resolveFrom) {
  try {
    return require.resolve(`${info.packageName}/package.json`, {
      paths: [resolveFrom],
    });
  } catch (error) {
    if (error && error.code === "MODULE_NOT_FOUND") {
      throw new Error(formatMissingPackageError(info, resolveFrom));
    }

    throw error;
  }
}

function resolveInstalledBinary(options = {}) {
  const platform = options.platform ?? process.platform;
  const arch = options.arch ?? process.arch;
  const resolveFrom = path.resolve(
    options.resolveFrom ?? path.join(__dirname, ".."),
  );
  const info = getPlatformInfo(platform, arch);

  if (!info) {
    throw new Error(
      [
        `Unsupported platform: ${platform}-${arch}.`,
        `Supported npm targets: ${getSupportedPlatforms().join(", ")}.`,
        "Build from source with Cargo instead.",
      ].join(" "),
    );
  }

  const packageJsonPath = resolvePlatformPackageJson(info, resolveFrom);
  const binaryPath = path.join(
    path.dirname(packageJsonPath),
    info.binaryRelativePath,
  );

  if (!fs.existsSync(binaryPath)) {
    throw new Error(
      [
        `The installed platform package ${info.packageName}@${info.version} is missing its binary.`,
        `Expected path: ${binaryPath}.`,
        "Reinstall the package or rebuild the npm platform artifact.",
      ].join(" "),
    );
  }

  return binaryPath;
}

async function ensureBinary(options = {}) {
  const overrideBinary = process.env.TELEGRAM_CLI_BINARY_PATH;
  if (overrideBinary) {
    const resolvedOverride = path.resolve(overrideBinary);
    if (!fs.existsSync(resolvedOverride)) {
      throw new Error(
        `TELEGRAM_CLI_BINARY_PATH does not exist: ${resolvedOverride}`,
      );
    }

    return resolvedOverride;
  }

  return resolveInstalledBinary(options);
}

async function bestEffortInstallBinary() {
  try {
    return await ensureBinary();
  } catch (error) {
    console.warn(`[telegram-agent-cli] ${error.message}`);
    return null;
  }
}

module.exports = {
  bestEffortInstallBinary,
  ensureBinary,
  formatUnsupportedPlatformError,
  resolveInstalledBinary,
};
