//! Library crate root re-exporting server and tool modules.

#[path = "lib/mod.rs"]
pub mod lib_mod;
pub use lib_mod as lib;
pub mod cli;
pub mod server;
pub mod tools;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn runtime_layout_requires_split_modules() {
        let expected_files = [
            "src/server/runtime/mod.rs",
            "src/server/runtime/startup.rs",
            "src/server/runtime/tool_registry.rs",
            "src/server/runtime/server_info.rs",
        ];

        for path in expected_files {
            assert!(
                Path::new(path).exists(),
                "runtime layout: {} must exist",
                path
            );
        }

        let mod_path = Path::new("src/server/runtime/mod.rs");
        let content = fs::read_to_string(mod_path)
            .unwrap_or_else(|_| panic!("runtime layout: failed to read {}", mod_path.display()));

        for needle in ["startup", "tool_registry", "server_info"] {
            assert!(
                content.contains(needle),
                "runtime layout: mod.rs must re-export {}",
                needle
            );
        }
    }

    #[test]
    fn visionos_build_layout_requires_split_modules() {
        let expected_files = [
            "src/tools/visionos/build/mod.rs",
            "src/tools/visionos/build/request.rs",
            "src/tools/visionos/build/executor.rs",
            "src/tools/visionos/build/queue.rs",
        ];

        for path in expected_files {
            assert!(
                Path::new(path).exists(),
                "visionOS build layout: {} must exist",
                path
            );
        }

        let mod_path = Path::new("src/tools/visionos/build/mod.rs");
        let content = fs::read_to_string(mod_path).unwrap_or_else(|_| {
            panic!(
                "visionOS build layout: failed to read {}",
                mod_path.display()
            )
        });

        for needle in ["request", "executor", "queue"] {
            assert!(
                content.contains(needle),
                "visionOS build layout: mod.rs must re-export {}",
                needle
            );
        }
    }

    #[test]
    fn cli_layout_requires_split_modules() {
        let expected_files = ["src/cli/mod.rs", "src/cli/args.rs", "src/cli/profile.rs"];

        for path in expected_files {
            assert!(Path::new(path).exists(), "CLI layout: {} must exist", path);
        }

        let mod_path = Path::new("src/cli/mod.rs");
        let content = fs::read_to_string(mod_path)
            .unwrap_or_else(|_| panic!("CLI layout: failed to read {}", mod_path.display()));

        assert!(
            content.contains("LaunchProfileArgs"),
            "CLI layout: mod.rs must re-export LaunchProfileArgs"
        );
    }

    #[test]
    fn config_layout_requires_split_modules() {
        let expected_files = [
            "src/server/config/mod.rs",
            "src/server/config/auth.rs",
            "src/server/config/server.rs",
            "src/server/config/visionos.rs",
            "src/server/config/telemetry.rs",
        ];

        for path in expected_files {
            assert!(
                Path::new(path).exists(),
                "config layout: {} must exist",
                path
            );
        }

        let mod_path = Path::new("src/server/config/mod.rs");
        let content = fs::read_to_string(mod_path)
            .unwrap_or_else(|_| panic!("config layout: failed to read {}", mod_path.display()));

        for needle in ["auth", "server", "visionos", "telemetry"] {
            assert!(
                content.contains(needle),
                "config layout: mod.rs must re-export {}",
                needle
            );
        }
    }
}
