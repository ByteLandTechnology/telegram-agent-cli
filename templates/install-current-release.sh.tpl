#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILL_NAME="{{SKILL_NAME}}"
VERSION="${1:-}"
INSTALL_DIR="${INSTALL_DIR:-${REPO_ROOT}/.local/bin}"
PLATFORM="$(uname -s)"
ARCH="$(uname -m)"

if [[ -z "${VERSION}" ]]; then
  if command -v git >/dev/null 2>&1; then
    VERSION="$(git -C "${REPO_ROOT}" describe --tags --exact-match 2>/dev/null || true)"
    VERSION="${VERSION#v}"
  fi
fi

if [[ -z "${VERSION}" ]]; then
  echo "Unable to determine release version. Check out a released tag or pass the version explicitly." >&2
  exit 1
fi

case "${PLATFORM}:${ARCH}" in
  Linux:x86_64)
    TARGET="x86_64-unknown-linux-gnu"
    ;;
  Darwin:arm64)
    TARGET="aarch64-apple-darwin"
    ;;
  *)
    echo "Unsupported platform ${PLATFORM}:${ARCH}. See the repo release notes for supported targets." >&2
    exit 1
    ;;
esac

ARCHIVE_NAME="${SKILL_NAME}-${VERSION}-${TARGET}.tar.gz"
RELEASE_URL="https://github.com/${GITHUB_REPOSITORY:-REPLACE_WITH_OWNER/REPO}/releases/tag/v${VERSION}"
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
tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"
install -m 0755 "${TMP_DIR}/${SKILL_NAME}" "${INSTALL_DIR}/${SKILL_NAME}"

echo "Installed ${SKILL_NAME} ${VERSION} to ${INSTALL_DIR}/${SKILL_NAME}"
echo "Verify with: ${INSTALL_DIR}/${SKILL_NAME} --version"
