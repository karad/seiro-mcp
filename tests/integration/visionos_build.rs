use std::{env, path::PathBuf, time::Duration};

use anyhow::Result;
use rmcp::{
    model::{CallToolRequestParam, ClientInfo},
    serve_client,
    service::ServiceError,
    ServiceExt,
};
use serde_json::{json, Value};
use tokio::time::Instant;
use uuid::Uuid;

use seiro_mcp::server::{
    config::{AuthSection, ServerConfig, ServerSection, VisionOsConfig},
    runtime::VisionOsServer,
};

#[tokio::test]
async fn build_tool_returns_artifact_metadata() -> Result<()> {
    enable_fast_timeout();
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
    .expect("JSON object")
    .clone();

    let start = Instant::now();
    let call_result = client
        .call_tool(CallToolRequestParam {
            name: "build_visionos_app".into(),
            arguments: Some(args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let response = call_result.expect("build_visionos_app should return a success response");
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "visionOS build tool must respond within 30 seconds (test environment)"
    );
    let payload = response
        .structured_content
        .expect("structured_content should exist");
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("succeeded")
    );
    assert!(payload
        .get("artifact_path")
        .and_then(|v| v.as_str())
        .is_some());
    assert!(payload
        .get("artifact_sha256")
        .and_then(|v| v.as_str())
        .is_some());
    assert!(payload.get("job_id").and_then(|v| v.as_str()).is_some());
    Ok(())
}

#[tokio::test]
async fn build_tool_times_out_when_process_exceeds_deadline() -> Result<()> {
    enable_fast_timeout();
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
    .expect("JSON object")
    .clone();

    let call_result = client
        .call_tool(CallToolRequestParam {
            name: "build_visionos_app".into(),
            arguments: Some(args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let error = call_result.expect_err("should return an error on timeout");
    match error {
        ServiceError::McpError(inner) => {
            assert_error_metadata(&inner, "timeout", "no_violation", true);
        }
        other => panic!("Unexpected error: {other:?}", other = other),
    }
    Ok(())
}

#[tokio::test]
async fn build_tool_rejects_path_outside_allowlist() -> Result<()> {
    enable_fast_timeout();
    let config = test_server_config(20);
    let server = build_server(config);
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        server.serve(server_transport).await?.waiting().await?;
        Result::<_, anyhow::Error>::Ok(())
    });
    let client = serve_client(ClientInfo::default(), client_transport).await?;

    let args = json!({
        "project_path": PathBuf::from("/tmp/disallowed-project").to_string_lossy(),
        "scheme": "VisionApp",
        "destination": "platform=visionOS Simulator,name=Apple Vision Pro"
    })
    .as_object()
    .expect("JSON object")
    .clone();

    let call_result = client
        .call_tool(CallToolRequestParam {
            name: "build_visionos_app".into(),
            arguments: Some(args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let error = call_result.expect_err("disallowed path should return an error");
    match error {
        ServiceError::McpError(inner) => {
            assert_error_metadata(&inner, "path_not_allowed", "blocked", false);
        }
        other => panic!("Unexpected error: {other:?}", other = other),
    }
    Ok(())
}

#[tokio::test]
async fn fetch_tool_returns_artifact_metadata() -> Result<()> {
    enable_fast_timeout();
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
    .expect("JSON object")
    .clone();

    let build_response = client
        .call_tool(CallToolRequestParam {
            name: "build_visionos_app".into(),
            arguments: Some(args),
        })
        .await
        .expect("build_visionos_app should succeed");
    let build_payload = build_response
        .structured_content
        .expect("structured_content");
    let job_id = build_payload
        .get("job_id")
        .and_then(|v| v.as_str())
        .expect("job_id")
        .to_string();

    let fetch_args = json!({
        "job_id": job_id.clone(),
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
        .expect("fetch_build_output should succeed");

    let _ = client.cancel().await;
    let _ = server_task.await;

    let payload = fetch_response
        .structured_content
        .expect("structured_content");
    assert_eq!(
        payload.get("status").and_then(|v| v.as_str()),
        Some("succeeded")
    );
    assert_eq!(
        payload.get("job_id").and_then(|v| v.as_str()),
        Some(job_id.as_str())
    );
    assert!(
        payload
            .get("artifact_zip")
            .and_then(|v| v.as_str())
            .is_some(),
        "artifact_zip should be present"
    );
    assert!(
        payload.get("sha256").and_then(|v| v.as_str()).is_some(),
        "sha256 should be present"
    );
    assert!(
        payload
            .get("download_ttl_seconds")
            .and_then(|v| v.as_u64())
            .is_some(),
        "download_ttl_seconds should be present"
    );
    Ok(())
}

#[tokio::test]
async fn fetch_tool_reports_expiration_after_ttl() -> Result<()> {
    enable_fast_timeout();
    let config = test_server_config_with_ttl(20, 1);
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
    .expect("JSON object")
    .clone();

    let build_response = client
        .call_tool(CallToolRequestParam {
            name: "build_visionos_app".into(),
            arguments: Some(args),
        })
        .await
        .expect("build_visionos_app should succeed");
    let build_payload = build_response
        .structured_content
        .expect("structured_content");
    let job_id = build_payload
        .get("job_id")
        .and_then(|v| v.as_str())
        .expect("job_id")
        .to_string();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let fetch_args = json!({ "job_id": job_id.clone() })
        .as_object()
        .expect("object")
        .clone();

    let fetch_result = client
        .call_tool(CallToolRequestParam {
            name: "fetch_build_output".into(),
            arguments: Some(fetch_args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let error = fetch_result.expect_err("artifact should expire");
    match error {
        ServiceError::McpError(inner) => {
            assert_error_metadata(&inner, "artifact_expired", "no_violation", true);
        }
        other => panic!("unexpected error: {other:?}", other = other),
    }
    Ok(())
}

#[tokio::test]
async fn fetch_tool_rejects_unknown_job() -> Result<()> {
    enable_fast_timeout();
    let config = test_server_config(20);
    let server = build_server(config);
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        server.serve(server_transport).await?.waiting().await?;
        Result::<_, anyhow::Error>::Ok(())
    });
    let client = serve_client(ClientInfo::default(), client_transport).await?;

    let fetch_args = json!({ "job_id": Uuid::new_v4().to_string() })
        .as_object()
        .expect("object")
        .clone();

    let fetch_result = client
        .call_tool(CallToolRequestParam {
            name: "fetch_build_output".into(),
            arguments: Some(fetch_args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let error = fetch_result.expect_err("unknown job should be rejected");
    match error {
        ServiceError::McpError(inner) => {
            assert_error_metadata(&inner, "job_not_found", "no_violation", false);
        }
        other => panic!("unexpected error: {other:?}", other = other),
    }
    Ok(())
}

#[tokio::test]
async fn sandbox_tool_reports_checks() -> Result<()> {
    enable_fast_timeout();
    configure_sandbox_probe_env();
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
    .expect("JSON object")
    .clone();

    let call_result = client
        .call_tool(CallToolRequestParam {
            name: "validate_sandbox_policy".into(),
            arguments: Some(args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let response = call_result.expect("validate_sandbox_policy should return a success response");
    let payload = response
        .structured_content
        .expect("structured_content should exist");
    assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert!(
        payload
            .get("checks")
            .and_then(|v| v.as_array())
            .map(|checks| !checks.is_empty())
            .unwrap_or(false),
        "checks must not be empty"
    );
    Ok(())
}

#[tokio::test]
async fn sandbox_tool_rejects_disallowed_path() -> Result<()> {
    enable_fast_timeout();
    configure_sandbox_probe_env();
    let config = test_server_config(20);
    let server = build_server(config);
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    let server_task = tokio::spawn(async move {
        server.serve(server_transport).await?.waiting().await?;
        Result::<_, anyhow::Error>::Ok(())
    });
    let client = serve_client(ClientInfo::default(), client_transport).await?;

    let args = json!({
        "project_path": "/tmp/disallowed-project",
        "required_sdks": ["visionOS"],
        "xcode_path": "/Applications/Xcode.app/Contents/Developer"
    })
    .as_object()
    .expect("JSON object")
    .clone();

    let call_result = client
        .call_tool(CallToolRequestParam {
            name: "validate_sandbox_policy".into(),
            arguments: Some(args),
        })
        .await;

    let _ = client.cancel().await;
    let _ = server_task.await;

    let error = call_result.expect_err("expected path_not_allowed error");
    match error {
        ServiceError::McpError(inner) => {
            assert_error_metadata(&inner, "path_not_allowed", "blocked", false);
        }
        other => panic!("Unexpected error: {other:?}", other = other),
    }
    Ok(())
}

fn allowed_project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visionos/workspace/VisionApp")
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

fn test_server_config_with_ttl(max_build_minutes: u16, ttl_secs: u32) -> ServerConfig {
    let mut config = test_server_config(max_build_minutes);
    config.visionos.artifact_ttl_secs = ttl_secs;
    config.visionos.cleanup_schedule_secs = 30;
    config
}

fn mock_xcodebuild_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/visionos/mock-xcodebuild.sh")
}

fn build_server(config: ServerConfig) -> VisionOsServer {
    VisionOsServer::new(config, "visionos-integration".into())
}

fn assert_error_metadata(
    error: &rmcp::model::ErrorData,
    expected_code: &str,
    expected_sandbox: &str,
    expected_retryable: bool,
) {
    let code = error_field(error, "code").and_then(Value::as_str);
    assert_eq!(code, Some(expected_code));
    let sandbox_state = error_field(error, "sandbox_state").and_then(Value::as_str);
    assert_eq!(sandbox_state, Some(expected_sandbox));
    let retryable = error_field(error, "retryable").and_then(Value::as_bool);
    assert_eq!(retryable, Some(expected_retryable));
    let remediation = error_field(error, "remediation")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        !remediation.is_empty(),
        "remediation should not be empty for code={expected_code}"
    );
}

fn error_field<'a>(error: &'a rmcp::model::ErrorData, key: &str) -> Option<&'a Value> {
    error.data.as_ref().and_then(|data| data.get(key))
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
