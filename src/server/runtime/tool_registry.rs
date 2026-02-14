use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use rmcp::{
    handler::server::{wrapper::Parameters, ServerHandler},
    model::{ErrorData, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, Json,
};
use uuid::Uuid;

use crate::{
    lib::errors::VisionOsBuildError,
    server::config::ServerConfig,
    tools::{
        self,
        visionos::{
            self, BuildVisionOsAppResponse, FetchBuildOutputRequest, FetchBuildOutputResponse,
            SandboxPolicyRequest, SandboxPolicyResponse, VisionOsArtifactStore,
            VisionOsBuildRequest, VisionOsJobQueue,
        },
        ServerToolRouter,
    },
};

#[derive(Clone)]
pub struct VisionOsServer {
    config: Arc<ServerConfig>,
    instructions: Arc<String>,
    tool_router: ServerToolRouter<Self>,
    visionos_queue: VisionOsJobQueue,
    artifact_store: VisionOsArtifactStore,
}

/// Compatibility alias to preserve the legacy `HelloWorldServer` name.
pub type HelloWorldServer = VisionOsServer;

impl VisionOsServer {
    pub fn new(config: ServerConfig, instructions: String) -> Self {
        let router = tools::build_router(Self::tool_router);
        let artifact_store = visionos::VisionOsArtifactStore::new(
            config.visionos.artifact_ttl_secs,
            config.visionos.cleanup_schedule_secs,
        );
        Self {
            config: Arc::new(config),
            instructions: Arc::new(instructions),
            tool_router: router,
            visionos_queue: VisionOsJobQueue::new(),
            artifact_store,
        }
    }

    pub async fn pending_jobs(&self) -> usize {
        self.visionos_queue.pending_jobs().await
    }

    async fn record_build_failure(&self, job_id: Uuid, err: &VisionOsBuildError) {
        let log_excerpt = match err {
            VisionOsBuildError::CommandFailed { message, .. } => message.clone(),
            _ => err.to_string(),
        };
        if let Err(store_err) = self
            .artifact_store
            .record_failure(job_id, log_excerpt, Utc::now())
            .await
        {
            tracing::warn!(
                target: "rmcp_sample::visionos",
                job_id = %job_id,
                error = %store_err,
                "Failed to record build failure"
            );
        }
    }
}

#[tool_router(router = tool_router)]
impl VisionOsServer {
    #[tool(
        name = "build_visionos_app",
        description = "Build a visionOS project and return artifact metadata"
    )]
    async fn build_visionos_app(
        &self,
        Parameters(request): Parameters<VisionOsBuildRequest>,
    ) -> Result<Json<BuildVisionOsAppResponse>, ErrorData> {
        if let Err(err) = request.validate(&self.config.visionos) {
            return Err(visionos::validation_error_to_error_data(err));
        }

        let job_id = Uuid::new_v4();
        let _ticket = self.visionos_queue.wait_for_turn(job_id).await;
        let result = visionos::run_build(
            &request,
            &self.config.visionos,
            job_id,
            self.artifact_store.root_dir(),
        )
        .await;
        self.visionos_queue.finish_job(job_id).await;

        match result {
            Ok(resp) => {
                if let Err(store_err) = self
                    .artifact_store
                    .record_success(
                        job_id,
                        PathBuf::from(&resp.artifact_path),
                        resp.artifact_sha256.clone(),
                        resp.log_excerpt.clone(),
                        Utc::now(),
                    )
                    .await
                {
                    let err = VisionOsBuildError::from(store_err);
                    return Err(visionos::runtime_error_to_error_data(err, job_id));
                }
                Ok(Json(resp))
            }
            Err(err) => {
                self.record_build_failure(job_id, &err).await;
                Err(visionos::runtime_error_to_error_data(err, job_id))
            }
        }
    }

    #[tool(
        name = "validate_sandbox_policy",
        description = "Validate allowed paths, SDKs, DevToolsSecurity, and related requirements"
    )]
    async fn validate_sandbox_policy(
        &self,
        Parameters(request): Parameters<SandboxPolicyRequest>,
    ) -> Result<Json<SandboxPolicyResponse>, ErrorData> {
        match visionos::validate_sandbox_policy(request, &self.config.visionos).await {
            Ok(response) => Ok(Json(response)),
            Err(err) => Err(visionos::sandbox_error_to_error_data(err)),
        }
    }

    #[tool(
        name = "fetch_build_output",
        description = "Fetch metadata for the latest visionOS build artifacts"
    )]
    async fn fetch_build_output(
        &self,
        Parameters(request): Parameters<FetchBuildOutputRequest>,
    ) -> Result<Json<FetchBuildOutputResponse>, ErrorData> {
        match visionos::fetch_build_output(&self.artifact_store, request).await {
            Ok(response) => Ok(Json(response)),
            Err(err) => Err(visionos::fetch_error_to_error_data(err)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for VisionOsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some((*self.instructions).clone()),
            ..ServerInfo::default()
        }
    }
}
