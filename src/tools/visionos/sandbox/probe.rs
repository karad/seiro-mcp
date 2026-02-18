use std::{
    collections::BTreeSet, ffi::CString, os::unix::ffi::OsStrExt, path::Path, path::PathBuf,
    process::Command,
};

use crate::lib::errors::SandboxPolicyError;

use super::MIN_DISK_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkInventory {
    pub raw: Vec<String>,
    pub normalized: Vec<String>,
    pub invocation: Option<String>,
    pub notes: Vec<String>,
}

fn normalize_sdks(raw_sdks: &[String]) -> Vec<String> {
    let mut normalized = BTreeSet::<String>::new();
    for sdk in raw_sdks {
        let trimmed = sdk.trim();
        if trimmed.is_empty() {
            continue;
        }
        normalized.insert(trimmed.to_string());
        let lower = trimmed.to_lowercase();
        if lower.starts_with("xros") || lower.starts_with("visionos") {
            normalized.insert("visionOS".to_string());
            normalized.insert("xrOS".to_string());
        }
        if lower.starts_with("xrsimulator") || lower.starts_with("visionossimulator") {
            normalized.insert("visionOS Simulator".to_string());
            normalized.insert("xrOS Simulator".to_string());
        }
    }
    normalized.into_iter().collect()
}

fn parse_showsdks_output(stdout: &str, invocation: Option<String>) -> SdkInventory {
    let mut raw = BTreeSet::<String>::new();
    for sdk in stdout.lines().filter_map(parse_sdk_from_showsdks_line) {
        raw.insert(sdk);
    }
    let raw: Vec<String> = raw.into_iter().collect();
    SdkInventory {
        normalized: normalize_sdks(&raw),
        raw,
        invocation,
        notes: Vec::new(),
    }
}

fn parse_sdk_from_showsdks_line(line: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for (idx, token) in tokens.iter().enumerate() {
        if *token == "-sdk" {
            return tokens.get(idx + 1).map(|sdk| (*sdk).to_string());
        }
        if let Some(sdk) = token.strip_prefix("-sdk") {
            let sdk = sdk.trim();
            if !sdk.is_empty() {
                return Some(sdk.to_string());
            }
        }
    }
    None
}

/// Abstraction for environment access during sandbox validation.
pub trait SandboxProbe {
    fn requires_developer_dir(&self) -> bool {
        true
    }
    fn list_sdks(&self, developer_dir: &Path) -> Result<SdkInventory, SandboxPolicyError>;
    fn devtools_security_enabled(&self) -> Result<bool, SandboxPolicyError>;
    fn xcode_license_accepted(&self) -> Result<bool, SandboxPolicyError>;
    fn disk_free_bytes(&self, path: &Path) -> Result<u64, SandboxPolicyError>;
}

/// Probe that operates against the real environment.
pub struct SystemSandboxProbe;

impl SandboxProbe for SystemSandboxProbe {
    fn list_sdks(&self, developer_dir: &Path) -> Result<SdkInventory, SandboxPolicyError> {
        let mut command = Command::new("xcodebuild");
        command.arg("-showsdks");
        if !developer_dir.as_os_str().is_empty() {
            command.env("DEVELOPER_DIR", developer_dir);
        }
        let invocation = if developer_dir.as_os_str().is_empty() {
            "xcodebuild -showsdks".to_string()
        } else {
            format!(
                "DEVELOPER_DIR={} xcodebuild -showsdks",
                developer_dir.display()
            )
        };
        let output = command
            .output()
            .map_err(|err| SandboxPolicyError::Internal {
                message: format!("Failed to run xcodebuild: {err}"),
            })?;
        if !output.status.success() {
            return Err(SandboxPolicyError::Internal {
                message: format!(
                    "xcodebuild -showsdks failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_showsdks_output(&stdout, Some(invocation)))
    }

    fn devtools_security_enabled(&self) -> Result<bool, SandboxPolicyError> {
        let output = Command::new("DevToolsSecurity")
            .arg("-status")
            .output()
            .map_err(|err| SandboxPolicyError::Internal {
                message: format!("Failed to run DevToolsSecurity: {err}"),
            })?;
        if !output.status.success() {
            return Err(SandboxPolicyError::DevToolsSecurityDisabled);
        }
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        Ok(stdout.contains("enabled"))
    }

    fn xcode_license_accepted(&self) -> Result<bool, SandboxPolicyError> {
        let status = Command::new("xcodebuild")
            .arg("-checkFirstLaunchStatus")
            .status()
            .map_err(|err| SandboxPolicyError::Internal {
                message: format!("xcodebuild -checkFirstLaunchStatus failed: {err}"),
            })?;
        if status.success() {
            Ok(true)
        } else {
            Err(SandboxPolicyError::LicenseNotAccepted)
        }
    }

    fn disk_free_bytes(&self, path: &Path) -> Result<u64, SandboxPolicyError> {
        let target = if path.exists() {
            path.to_path_buf()
        } else {
            PathBuf::from("/")
        };
        let c_path = CString::new(target.as_os_str().as_bytes()).map_err(|err| {
            SandboxPolicyError::Internal {
                message: format!("Failed to parse disk path: {err}"),
            }
        })?;
        let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
        let result = unsafe { libc::statfs(c_path.as_ptr(), stats.as_mut_ptr()) };
        if result != 0 {
            return Err(SandboxPolicyError::Internal {
                message: "statfs call failed".into(),
            });
        }
        let stats = unsafe { stats.assume_init() };

        #[cfg(target_os = "linux")]
        let available_blocks = stats.f_bavail;
        #[cfg(target_os = "macos")]
        let available_blocks = stats.f_bavail;
        #[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
        let available_blocks = stats.f_bavail as u64;

        #[cfg(target_os = "linux")]
        let block_size =
            u64::try_from(stats.f_bsize).map_err(|_| SandboxPolicyError::Internal {
                message: format!("statfs returned negative block size: {}", stats.f_bsize),
            })?;
        #[cfg(target_os = "macos")]
        let block_size = u64::from(stats.f_bsize);
        #[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
        let block_size = stats.f_bsize as u64;

        available_blocks
            .checked_mul(block_size)
            .ok_or_else(|| SandboxPolicyError::Internal {
                message: "statfs overflow when computing free bytes".into(),
            })
    }
}

pub struct EnvSandboxProbe;

impl SandboxProbe for EnvSandboxProbe {
    fn requires_developer_dir(&self) -> bool {
        false
    }

    fn list_sdks(&self, _developer_dir: &Path) -> Result<SdkInventory, SandboxPolicyError> {
        let sdks = std::env::var("VISIONOS_SANDBOX_SDKS").unwrap_or_default();
        let raw: Vec<String> = sdks
            .split(',')
            .filter_map(|entry| {
                let trimmed = entry.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();
        let mut notes = Vec::new();
        if raw.is_empty() {
            notes.push(
                "VISIONOS_SANDBOX_SDKS is empty; env probe does not execute xcodebuild".into(),
            );
        }
        Ok(SdkInventory {
            normalized: normalize_sdks(&raw),
            raw,
            invocation: None,
            notes,
        })
    }

    fn devtools_security_enabled(&self) -> Result<bool, SandboxPolicyError> {
        Ok(matches!(
            std::env::var("VISIONOS_SANDBOX_DEVTOOLS")
                .unwrap_or_else(|_| "enabled".into())
                .to_lowercase()
                .as_str(),
            "enabled" | "true" | "1"
        ))
    }

    fn xcode_license_accepted(&self) -> Result<bool, SandboxPolicyError> {
        Ok(matches!(
            std::env::var("VISIONOS_SANDBOX_LICENSE")
                .unwrap_or_else(|_| "accepted".into())
                .to_lowercase()
                .as_str(),
            "accepted" | "true" | "1"
        ))
    }

    fn disk_free_bytes(&self, _path: &Path) -> Result<u64, SandboxPolicyError> {
        let bytes = std::env::var("VISIONOS_SANDBOX_DISK_BYTES")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(u64::MAX / 2);

        if bytes < MIN_DISK_BYTES {
            return Err(SandboxPolicyError::DiskInsufficient {
                available_bytes: bytes,
            });
        }

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_sdk_from_showsdks_line, parse_showsdks_output};

    #[test]
    fn parse_sdk_from_showsdks_line_extracts_sdk_from_two_token_form() {
        let line = "    visionOS 26.0                   -sdk xros26.0";
        assert_eq!(
            parse_sdk_from_showsdks_line(line),
            Some("xros26.0".to_string())
        );
    }

    #[test]
    fn parse_showsdks_output_collects_multiple_sdks() {
        let stdout = r#"
iOS SDKs:
    iOS 18.0                       -sdk iphoneos18.0

visionOS SDKs:
    visionOS 26.0                   -sdk xros26.0
    Simulator - visionOS 26.0       -sdk xrsimulator26.0
"#;
        let inventory = parse_showsdks_output(stdout, Some("xcodebuild -showsdks".into()));
        assert!(inventory.raw.contains(&"iphoneos18.0".to_string()));
        assert!(inventory.raw.contains(&"xros26.0".to_string()));
        assert!(inventory.raw.contains(&"xrsimulator26.0".to_string()));
        assert!(inventory.normalized.contains(&"visionOS".to_string()));
        assert!(inventory
            .normalized
            .contains(&"visionOS Simulator".to_string()));
        assert_eq!(
            inventory.invocation.as_deref(),
            Some("xcodebuild -showsdks")
        );
    }
}
