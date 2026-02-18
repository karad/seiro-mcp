use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, ClientInfo},
    serve_client,
    service::ServiceError,
    ServiceExt,
};
use serde_json::{json, Value};

use seiro_mcp::server::{
    config::{AuthSection, ServerConfig, ServerSection, VisionOsConfig},
    runtime::VisionOsServer,
};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn write_fixture(path: &PathBuf, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create fixture dir {}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(value).context("failed to serialize fixture")?;
    fs::write(path, format!("{serialized}\n"))
        .with_context(|| format!("failed to write fixture {}", path.display()))?;
    Ok(())
}

fn assert_json_fixture(name: &str, actual: &Value) -> Result<()> {
    let path = fixture_path(&format!("tests/fixtures/refactor/{name}.json"));
    if env::var("UPDATE_FIXTURES").ok().as_deref() == Some("1") {
        write_fixture(&path, actual)?;
        return Ok(());
    }
    let expected =
        fs::read_to_string(&path).with_context(|| format!("missing fixture {}", path.display()))?;
    let expected: Value = serde_json::from_str(&expected)
        .with_context(|| format!("invalid JSON {}", path.display()))?;
    assert_eq!(actual, &expected);
    Ok(())
}

fn enable_fast_timeout() {
    env::set_var("VISIONOS_TEST_TIME_SCALE", "1");
}

fn configure_sandbox_probe_env() {
    env::set_var("VISIONOS_SANDBOX_PROBE", "env");
    env::set_var("VISIONOS_SANDBOX_SDKS", "visionOS,visionOS Simulator");
    env::set_var("VISIONOS_SANDBOX_DEVTOOLS", "enabled");
    env::set_var("VISIONOS_SANDBOX_LICENSE", "accepted");
    env::set_var("VISIONOS_SANDBOX_DISK_BYTES", "1099511627776");
}

fn allowed_project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visionos/workspace/VisionApp")
}

fn mock_xcodebuild_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visionos/mock-xcodebuild.sh")
}

fn test_server_config(max_build_minutes: u16) -> ServerConfig {
    let workspace =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visionos/workspace");
    ServerConfig {
        server: ServerSection {
            host: "127.0.0.1".into(),
            port: 8787,
        },
        auth: AuthSection {
            token: "test-token".into(),
        },
        visionos: VisionOsConfig {
            allowed_paths: vec![workspace],
            allowed_schemes: vec!["VisionApp".into()],
            default_destination: "platform=visionOS Simulator,name=Apple Vision Pro".into(),
            required_sdks: vec!["visionOS".into(), "visionOS Simulator".into()],
            xcode_path: PathBuf::from("/Applications/Xcode.app/Contents/Developer"),
            xcodebuild_path: mock_xcodebuild_path(),
            max_build_minutes,
            artifact_ttl_secs: 600,
            cleanup_schedule_secs: 60,
        },
        source_path: PathBuf::from("tests/fixtures/config_valid.toml"),
    }
}

fn build_server(config: ServerConfig) -> VisionOsServer {
    VisionOsServer::new(config, "visionos-integration".into())
}

fn normalize_build_success(mut payload: Value) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        if obj.contains_key("job_id") {
            obj.insert("job_id".into(), Value::String("<job_id>".into()));
        }
        if obj.contains_key("artifact_path") {
            obj.insert(
                "artifact_path".into(),
                Value::String("<artifact_path>".into()),
            );
        }
        if obj.contains_key("artifact_sha256") {
            obj.insert(
                "artifact_sha256".into(),
                Value::String("<artifact_sha256>".into()),
            );
        }
        if obj.contains_key("duration_ms") {
            obj.insert("duration_ms".into(), Value::Number(0.into()));
        }
        if obj.contains_key("log_excerpt") {
            obj.insert("log_excerpt".into(), Value::String("<log_excerpt>".into()));
        }
    }
    payload
}

fn normalize_fetch_success(mut payload: Value) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        if obj.contains_key("job_id") {
            obj.insert("job_id".into(), Value::String("<job_id>".into()));
        }
        if obj.contains_key("artifact_zip") {
            obj.insert(
                "artifact_zip".into(),
                Value::String("<artifact_zip>".into()),
            );
        }
        if obj.contains_key("sha256") {
            obj.insert("sha256".into(), Value::String("<artifact_sha256>".into()));
        }
        if obj.contains_key("download_ttl_seconds") {
            obj.insert("download_ttl_seconds".into(), Value::Number(0.into()));
        }
        if obj.contains_key("log_excerpt") {
            obj.insert("log_excerpt".into(), Value::String("<log_excerpt>".into()));
        }
    }
    payload
}

fn normalize_sandbox_success(mut payload: Value) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        if let Some(checks) = obj.get_mut("checks").and_then(Value::as_array_mut) {
            for check in checks {
                if let Some(check_obj) = check.as_object_mut() {
                    if check_obj.contains_key("details") {
                        check_obj.insert("details".into(), Value::String("<details>".into()));
                    }
                }
            }
        }
        // Keep snapshot compatibility when additive diagnostics fields are introduced.
        obj.remove("diagnostics");
    }
    payload
}

fn normalize_error(mut error: Value) -> Value {
    if let Some(obj) = error.as_object_mut() {
        if obj.contains_key("message") {
            obj.insert("message".into(), Value::String("<message>".into()));
        }
        if let Some(data) = obj.get_mut("data").and_then(Value::as_object_mut) {
            if let Some(job_id) = data.get_mut("job_id") {
                if job_id.is_string() {
                    *job_id = Value::String("<job_id>".into());
                }
            }
            if data.contains_key("details") {
                data.insert("details".into(), Value::String("<details>".into()));
            }
            if let Some(path_val) = data.get_mut("path") {
                if path_val.is_string() {
                    *path_val = Value::String("<path>".into());
                }
            }
            if let Some(reason_val) = data.get_mut("reason") {
                if reason_val.is_string() {
                    *reason_val = Value::String("<reason>".into());
                }
            }
        }
    }
    error
}

#[tokio::test]
async fn refactor_behaviour_snapshots_match() -> Result<()> {
    enable_fast_timeout();
    configure_sandbox_probe_env();

    // build success snapshot
    {
        let config = test_server_config(20);
        let server = build_server(config);
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<_, anyhow::Error>::Ok(())
        });
        let client = serve_client(ClientInfo::default(), client_transport).await?;

        let args = json!({
            "project_path": allowed_project_path().to_string_lossy(),
            "scheme": "VisionApp",
            "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
            "env_overrides": {
                "MOCK_XCODEBUILD_BEHAVIOR": "success"
            }
        })
        .as_object()
        .expect("object")
        .clone();

        let response = client
            .call_tool(CallToolRequestParam {
                name: "build_visionos_app".into(),
                arguments: Some(args),
            })
            .await
            .expect("build should succeed");
        let _ = client.cancel().await;
        let _ = server_task.await;

        let payload = response.structured_content.expect("structured_content");
        assert_json_fixture("build_success", &normalize_build_success(payload))?;
    }

    // build timeout error snapshot
    {
        let config = test_server_config(1);
        let server = build_server(config);
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<_, anyhow::Error>::Ok(())
        });
        let client = serve_client(ClientInfo::default(), client_transport).await?;

        let args = json!({
            "project_path": allowed_project_path().to_string_lossy(),
            "scheme": "VisionApp",
            "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
            "env_overrides": {
                "MOCK_XCODEBUILD_BEHAVIOR": "sleep"
            }
        })
        .as_object()
        .expect("object")
        .clone();

        let call_result = client
            .call_tool(CallToolRequestParam {
                name: "build_visionos_app".into(),
                arguments: Some(args),
            })
            .await;
        let _ = client.cancel().await;
        let _ = server_task.await;

        let error = call_result.expect_err("timeout should error");
        let error_data = match error {
            ServiceError::McpError(inner) => serde_json::to_value(inner)?,
            other => anyhow::bail!("unexpected error: {other:?}"),
        };
        assert_json_fixture("build_timeout_error", &normalize_error(error_data))?;
    }

    // fetch success snapshot
    {
        let config = test_server_config(20);
        let server = build_server(config);
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<_, anyhow::Error>::Ok(())
        });
        let client = serve_client(ClientInfo::default(), client_transport).await?;

        let build_args = json!({
            "project_path": allowed_project_path().to_string_lossy(),
            "scheme": "VisionApp",
            "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
            "env_overrides": {
                "MOCK_XCODEBUILD_BEHAVIOR": "success"
            }
        })
        .as_object()
        .expect("object")
        .clone();

        let build_response = client
            .call_tool(CallToolRequestParam {
                name: "build_visionos_app".into(),
                arguments: Some(build_args),
            })
            .await
            .expect("build should succeed");
        let build_payload = build_response
            .structured_content
            .expect("structured_content");
        let job_id = build_payload
            .get("job_id")
            .and_then(Value::as_str)
            .context("job_id missing")?
            .to_string();

        let fetch_args = json!({
            "job_id": job_id,
            "include_logs": true
        })
        .as_object()
        .expect("object")
        .clone();

        let fetch_response = client
            .call_tool(CallToolRequestParam {
                name: "fetch_build_output".into(),
                arguments: Some(fetch_args),
            })
            .await
            .expect("fetch should succeed");
        let _ = client.cancel().await;
        let _ = server_task.await;

        let payload = fetch_response
            .structured_content
            .expect("structured_content");
        assert_json_fixture("fetch_success", &normalize_fetch_success(payload))?;
    }

    // sandbox success snapshot
    {
        let config = test_server_config(20);
        let server = build_server(config);
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<_, anyhow::Error>::Ok(())
        });
        let client = serve_client(ClientInfo::default(), client_transport).await?;

        let args = json!({
            "project_path": allowed_project_path().to_string_lossy(),
            "required_sdks": ["visionOS", "visionOS Simulator"],
            "xcode_path": "/Applications/Xcode.app/Contents/Developer"
        })
        .as_object()
        .expect("object")
        .clone();

        let response = client
            .call_tool(CallToolRequestParam {
                name: "validate_sandbox_policy".into(),
                arguments: Some(args),
            })
            .await
            .expect("sandbox should succeed");
        let _ = client.cancel().await;
        let _ = server_task.await;

        let payload = response.structured_content.expect("structured_content");
        assert_json_fixture("sandbox_success", &normalize_sandbox_success(payload))?;
    }

    Ok(())
}
