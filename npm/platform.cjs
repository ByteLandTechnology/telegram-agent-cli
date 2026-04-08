const pkg = require("../package.json");

const BINARY_NAME = "telegram-agent-cli";
const TARGETS = {
  "darwin:arm64": {
    packageName: "telegram-agent-cli-darwin-arm64",
    target: "aarch64-apple-darwin",
  },
  "darwin:x64": {
    packageName: "telegram-agent-cli-darwin-x64",
    target: "x86_64-apple-darwin",
  },
  "linux:arm64": {
    packageName: "telegram-agent-cli-linux-arm64",
    target: "aarch64-unknown-linux-gnu",
  },
  "linux:x64": {
    packageName: "telegram-agent-cli-linux-x64",
    target: "x86_64-unknown-linux-gnu",
  },
  "win32:arm64": {
    packageName: "telegram-agent-cli-windows-arm64",
    target: "aarch64-pc-windows-gnullvm",
  },
  "win32:x64": {
    packageName: "telegram-agent-cli-windows-x64",
    target: "x86_64-pc-windows-gnullvm",
  },
};

function getSupportedPlatforms() {
  return Object.keys(TARGETS)
    .map((key) => key.replace("win32", "windows").replace(":", "-"))
    .sort();
}

function getPlatformInfo(platform = process.platform, arch = process.arch) {
  const targetInfo = TARGETS[`${platform}:${arch}`];
  if (!targetInfo) {
    return null;
  }

  const archiveExtension = targetInfo.target.includes("windows")
    ? "zip"
    : "tar.gz";
  const archiveName = `${BINARY_NAME}-${pkg.version}-${targetInfo.target}.${archiveExtension}`;
  const binaryFileName =
    platform === "win32" ? `${BINARY_NAME}.exe` : BINARY_NAME;

  return {
    platform,
    arch,
    target: targetInfo.target,
    version: pkg.version,
    binaryName: BINARY_NAME,
    binaryFileName,
    binaryRelativePath: `bin/${binaryFileName}`,
    archiveName,
    archiveExtension,
    packageName: targetInfo.packageName,
  };
}

function getSupportedPackageNames() {
  return Object.values(TARGETS)
    .map((entry) => entry.packageName)
    .sort();
}

function getTargetInfos() {
  return Object.values(TARGETS)
    .map((entry) => ({
      archiveExtension: entry.target.includes("windows") ? "zip" : "tar.gz",
      binaryFileName: entry.target.includes("windows")
        ? `${BINARY_NAME}.exe`
        : BINARY_NAME,
      binaryName: BINARY_NAME,
      packageName: entry.packageName,
      target: entry.target,
    }))
    .sort((left, right) => left.target.localeCompare(right.target));
}

module.exports = {
  BINARY_NAME,
  getPlatformInfo,
  getSupportedPackageNames,
  getSupportedPlatforms,
  getTargetInfos,
};
