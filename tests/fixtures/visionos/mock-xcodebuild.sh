#!/usr/bin/env bash
set -euo pipefail

echo "[mock-xcodebuild] invoked with args: $*" >&2

ARTIFACT_DIR="${VISIONOS_BUILD_ARTIFACT_DIR:-}"
if [[ -z "${ARTIFACT_DIR}" ]]; then
  echo "[mock-xcodebuild] VISIONOS_BUILD_ARTIFACT_DIR is not set" >&2
  exit 2
fi

mkdir -p "${ARTIFACT_DIR}"

case "${MOCK_XCODEBUILD_BEHAVIOR:-success}" in
  sleep)
    # Simulate a long-running build that should hit the MCP timeout quickly.
    sleep 2
    ;;
  fail)
    echo "[mock-xcodebuild] simulated failure" >&2
    exit 65
    ;;
  *)
    echo "[mock-xcodebuild] generating dummy artifacts in ${ARTIFACT_DIR}" >&2
    mkdir -p "${ARTIFACT_DIR}/VisionApp.app"
    printf "dummy app bundle" > "${ARTIFACT_DIR}/VisionApp.app/Info.plist"
    mkdir -p "${ARTIFACT_DIR}/VisionApp.dSYM"
    printf "dummy dSYM" > "${ARTIFACT_DIR}/VisionApp.dSYM/Contents"
    ;;
esac
