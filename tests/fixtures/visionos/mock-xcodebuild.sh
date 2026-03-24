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
  ambiguous_destination)
    cat >&2 <<'EOF'
Command line invocation:
 /Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild -scheme VisionApp -configuration Debug -destination "platform=visionOS Simulator,name=Apple Vision Pro" build

xcodebuild: error: Unable to find a device matching the provided destination specifier:
		{ platform:visionOS Simulator, OS:latest, name:Apple Vision Pro }

	The requested device could not be found because multiple devices matched the request. (
 "<DVTiPhoneSimulator: 0xb57503480> {\n\t\tSimDevice: Apple Vision Pro (5BB47C97-BDBA-4DA7-BE30-F659C265F896, visionOS 2.5, Shutdown)\n}",
 "<DVTiPhoneSimulator: 0xb57503980> {\n\t\tSimDevice: Apple Vision Pro (F556D53F-412A-4778-AF81-3449D52F5A7F, visionOS 26.2, Shutdown)\n}"
)

	Available destinations for the "VisionApp" scheme:
		{ platform:visionOS, id:dvtdevice-DVTiOSDevicePlaceholder-xros:placeholder, name:Any visionOS Device }
		{ platform:visionOS Simulator, id:dvtdevice-DVTiOSDeviceSimulatorPlaceholder-xrsimulator:placeholder, name:Any visionOS Simulator Device }
		{ platform:visionOS Simulator, arch:arm64, id:F556D53F-412A-4778-AF81-3449D52F5A7F, OS:26.2, name:Apple Vision Pro }
EOF
    exit 70
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
