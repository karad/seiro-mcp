#!/usr/bin/env bash
set -euo pipefail

echo "[mock-xcodebuild] invoked with args: $*" >&2

if [[ "${1:-}" == "-list" || "${2:-}" == "-list" || "${3:-}" == "-list" ]]; then
  case "${MOCK_XCODEBUILD_BEHAVIOR:-success}" in
    fail)
      echo "[mock-xcodebuild] simulated list failure" >&2
      exit 65
      ;;
    parse_invalid)
      echo "{invalid-json"
      exit 0
      ;;
    no_schemes)
      cat <<'JSON'
{
  "project": {
    "name": "VisionApp",
    "schemes": []
  }
}
JSON
      exit 0
      ;;
    *)
      cat <<'JSON'
{
  "project": {
    "name": "VisionApp",
    "schemes": ["VisionApp", "VisionAppTests"]
  }
}
JSON
      exit 0
      ;;
  esac
fi

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
