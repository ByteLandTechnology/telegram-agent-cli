#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG_PATH="${REPO_ROOT}/release/skill-release.config.json"

if [[ ! -f "${CONFIG_PATH}" ]]; then
  echo "Missing ${CONFIG_PATH}. The release asset pack must be configured before install." >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Node.js is required to read ${CONFIG_PATH}." >&2
  exit 1
fi

mapfile -t RELEASE_INFO < <(
  node --input-type=module -e '
    import fs from "node:fs";
    const config = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const repo =
      process.env.GITHUB_REPOSITORY ||
      config.githubRelease?.ownerRepository ||
      config.sourceRepository ||
      "";
    console.log(config.sourceSkillId);
    console.log(repo);
  ' "${CONFIG_PATH}"
)

SKILL_NAME="${RELEASE_INFO[0]:-}"
OWNER_REPOSITORY="${RELEASE_INFO[1]:-}"
VERSION="${1:-}"
INSTALL_DIR="${INSTALL_DIR:-${REPO_ROOT}/.local/bin}"
PLATFORM="$(uname -s)"
ARCH="$(uname -m)"
BINARY_FILENAME="${SKILL_NAME}"
ARCHIVE_EXTENSION="tar.gz"

if [[ -z "${SKILL_NAME}" || -z "${OWNER_REPOSITORY}" || "${OWNER_REPOSITORY}" == *"REPLACE_WITH_OWNER/REPO"* ]]; then
  echo "release/skill-release.config.json must define the repository owner/name and skill id before install." >&2
  exit 1
fi

if [[ -z "${VERSION}" ]] && command -v git >/dev/null 2>&1; then
  VERSION="$(git -C "${REPO_ROOT}" describe --tags --exact-match 2>/dev/null || true)"
  VERSION="${VERSION#v}"
fi

if [[ -z "${VERSION}" ]]; then
  echo "Unable to determine release version. Check out a released tag or pass the version explicitly." >&2
  exit 1
fi

case "${PLATFORM}:${ARCH}" in
  Linux:x86_64)
    TARGET="x86_64-unknown-linux-gnu"
    ;;
  Linux:aarch64|Linux:arm64)
    TARGET="aarch64-unknown-linux-gnu"
    ;;
  Darwin:arm64)
    TARGET="aarch64-apple-darwin"
    ;;
  Darwin:x86_64)
    TARGET="x86_64-apple-darwin"
    ;;
  MINGW64_NT-*:x86_64|MSYS_NT-*:x86_64|CYGWIN_NT-*:x86_64)
    TARGET="x86_64-pc-windows-gnullvm"
    ARCHIVE_EXTENSION="zip"
    BINARY_FILENAME="${SKILL_NAME}.exe"
    ;;
  MINGW64_NT-*:aarch64|MSYS_NT-*:aarch64|CYGWIN_NT-*:aarch64|MINGW64_NT-*:arm64|MSYS_NT-*:arm64|CYGWIN_NT-*:arm64)
    TARGET="aarch64-pc-windows-gnullvm"
    ARCHIVE_EXTENSION="zip"
    BINARY_FILENAME="${SKILL_NAME}.exe"
    ;;
  *)
    echo "Unsupported platform ${PLATFORM}:${ARCH}. See the repo release notes for supported targets." >&2
    exit 1
    ;;
esac

ARCHIVE_NAME="${SKILL_NAME}-${VERSION}-${TARGET}.${ARCHIVE_EXTENSION}"
RELEASE_URL="https://github.com/${OWNER_REPOSITORY}/releases/tag/v${VERSION}"
DOWNLOAD_URL="${RELEASE_URL}/download/${ARCHIVE_NAME}"
TMP_DIR="$(mktemp -d)"
ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE_NAME}"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

mkdir -p "${INSTALL_DIR}"

echo "Downloading ${DOWNLOAD_URL}"
curl --fail --location --silent --show-error "${DOWNLOAD_URL}" -o "${ARCHIVE_PATH}"

if [[ "${ARCHIVE_EXTENSION}" == "zip" ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "${ARCHIVE_PATH}" -d "${TMP_DIR}"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "${ARCHIVE_PATH}" "${TMP_DIR}"
  elif command -v powershell.exe >/dev/null 2>&1; then
    powershell.exe -NoProfile -Command "Expand-Archive -LiteralPath '${ARCHIVE_PATH}' -DestinationPath '${TMP_DIR}' -Force" >/dev/null
  elif command -v pwsh >/dev/null 2>&1; then
    pwsh -NoProfile -Command "Expand-Archive -LiteralPath '${ARCHIVE_PATH}' -DestinationPath '${TMP_DIR}' -Force" >/dev/null
  else
    echo "Unable to extract ${ARCHIVE_NAME}; install unzip, python3, powershell.exe, or pwsh." >&2
    exit 1
  fi
else
  tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"
fi

install -m 0755 "${TMP_DIR}/${BINARY_FILENAME}" "${INSTALL_DIR}/${BINARY_FILENAME}"

echo "Installed ${SKILL_NAME} ${VERSION} to ${INSTALL_DIR}/${BINARY_FILENAME}"
echo "Verify with: ${INSTALL_DIR}/${BINARY_FILENAME} --version"
