use async_trait::async_trait;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{
    CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion,
    ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use toad_core::{GlobalConfig, Workspace};

#[derive(Clone)]
pub struct ToadService {
    pub tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListProjectsParams {
    /// Filter by project name (substring)
    pub query: Option<String>,
    /// Filter by tag (e.g., #backend)
    pub tag: Option<String>,
    /// Filter by stack name
    pub stack: Option<String>,
    /// Filter by activity tier
    pub activity: Option<String>,
    /// Filter by VCS status
    pub vcs_status: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetProjectDetailParams {
    /// Exact project name
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchProjectsParams {
    /// Search term
    pub query: String,
    /// Narrow search by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetEcosystemSummaryParams {
    /// Max tokens (default from config)
    pub token_limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetEcosystemStatusParams {
    /// Filter by project name (substring)
    pub query: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetProjectStatsParams {
    /// Filter by project name (substring)
    pub query: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SwitchContextParams {
    /// Name of the context to switch to
    pub name: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct NoParams {
    // Empty
}

#[derive(Deserialize, JsonSchema)]
pub struct CompareProjectsParams {
    /// Source project name
    pub source: String,
    /// Target project name
    pub target: String,
}

#[tool_router]
impl ToadService {
    pub fn new() -> anyhow::Result<Self> {
        // Verify we can discover a workspace at startup
        let _ = Workspace::discover()?;
        Ok(Self {
            tool_router: Self::tool_router(),
        })
    }

    #[tool(description = "List projects in the ecosystem, optionally filtered")]
    pub async fn list_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ListProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        let q_filter: Option<String> = params.query.map(|s| s.to_lowercase());
        let t_filter: Option<String> = params.tag.map(|s| {
            if s.starts_with('#') {
                s.to_lowercase()
            } else {
                format!("#{}", s.to_lowercase())
            }
        });
        let s_filter: Option<String> = params.stack.map(|s| s.to_lowercase());
        let a_filter: Option<String> = params.activity.map(|s| s.to_lowercase());
        let v_filter: Option<String> = params.vcs_status.map(|s| s.to_lowercase());

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let filtered: Vec<_> = registry
                .projects
                .into_iter()
                .filter(|p| {
                    if let Some(q) = &q_filter
                        && !p.name.to_lowercase().contains(q)
                    {
                        return false;
                    }
                    if let Some(t) = &t_filter
                        && !p.tags.iter().any(|tag| tag.to_lowercase() == *t)
                    {
                        return false;
                    }
                    if let Some(s) = &s_filter
                        && !p.stack.to_lowercase().contains(s)
                    {
                        return false;
                    }
                    if let Some(a) = &a_filter
                        && !p.activity.to_string().to_lowercase().contains(a)
                    {
                        return false;
                    }
                    if let Some(v) = &v_filter
                        && !p.vcs_status.to_string().to_lowercase().contains(v)
                    {
                        return false;
                    }
                    true
                })
                .collect();

            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&filtered)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get full context for a single project by name")]
    pub async fn get_project_detail(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = params.0.name;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let project = registry
                .projects
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| {
                    toad_core::ToadError::Other(format!("Project '{}' not found", name))
                })?;

            let mut output = serde_json::to_value(project)?;

            // Try to read CONTEXT.md if it exists
            let context_md_path = ws.shadows_dir.join(&name).join("CONTEXT.md");
            if context_md_path.exists()
                && let Ok(content) = fs::read_to_string(context_md_path)
            {
                output["context_md"] = serde_json::Value::String(content);
            }

            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&output)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get structural DNA for a single project")]
    pub async fn get_project_dna(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = params.0.name;

        let result = tokio::task::spawn_blocking(move || {
            let _ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(_ws.active_context.as_deref(), None)?;

            let project = registry
                .projects
                .iter()
                .find(|p| p.name == name)
                .ok_or_else(|| {
                    toad_core::ToadError::Other(format!("Project '{}' not found", name))
                })?;

            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&project.dna)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Compare two projects for migration compatibility")]
    pub async fn compare_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<CompareProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let source = params.0.source;
        let target = params.0.target;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let proj_a = registry
                .projects
                .iter()
                .find(|p| p.name == source)
                .ok_or_else(|| {
                    toad_core::ToadError::Other(format!("Source project '{}' not found", source))
                })?;
            let proj_b = registry
                .projects
                .iter()
                .find(|p| p.name == target)
                .ok_or_else(|| {
                    toad_core::ToadError::Other(format!("Target project '{}' not found", target))
                })?;

            let preflight = toad_ops::migration::compare_projects(proj_a, proj_b);
            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&preflight)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Search projects by DNA characteristics (role, capability, pattern)")]
    pub async fn search_projects_by_dna(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query.to_lowercase();

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let matches: Vec<_> = registry
                .projects
                .into_iter()
                .filter(|p| {
                    p.dna
                        .roles
                        .iter()
                        .any(|r| r.to_lowercase().contains(&query))
                        || p.dna
                            .capabilities
                            .iter()
                            .any(|c| c.to_lowercase().contains(&query))
                        || p.dna
                            .structural_patterns
                            .iter()
                            .any(|sp| sp.to_lowercase().contains(&query))
                })
                .collect();

            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&matches)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Semantic search across projects")]
    pub async fn search_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query;
        let tag = params.0.tag;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let search_result = toad_discovery::search_projects(&ws, &query, tag.as_deref())?;
            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&search_result)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get ecosystem summary (SYSTEM_PROMPT.md)")]
    pub async fn get_ecosystem_summary(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetEcosystemSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        let token_limit = params.0.token_limit;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let config = toad_core::GlobalConfig::load(None)?.unwrap_or_default();
            let limit = token_limit.or(Some(config.budget.ecosystem_tokens));

            let summary = toad_manifest::generate_system_prompt(&registry.projects, limit);
            Ok::<_, toad_core::ToadError>(summary)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get ecosystem health status")]
    pub async fn get_ecosystem_status(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetEcosystemStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query;
        let tag = params.0.tag;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let report =
                toad_discovery::generate_status_report(&ws, query.as_deref(), tag.as_deref())?;
            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get project disk usage stats")]
    pub async fn get_project_stats(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectStatsParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query;
        let tag = params.0.tag;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let report = toad_ops::stats::generate_analytics_report(
                &registry.projects,
                query.as_deref(),
                tag.as_deref(),
            );
            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Get the currently active project context")]
    pub async fn get_active_context(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tokio::task::spawn_blocking(move || {
            let config = GlobalConfig::load(None)?.unwrap_or_default();
            let active = config
                .active_context
                .unwrap_or_else(|| "default".to_string());
            let ctx = config.project_contexts.get(&active);

            Ok::<_, toad_core::ToadError>(serde_json::json!({
                "name": active,
                "path": ctx.map(|c| c.path.clone()),
                "type": ctx.map(|c| c.context_type.to_string()),
            }))
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(description = "List all registered project contexts")]
    pub async fn list_contexts(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tokio::task::spawn_blocking(move || {
            let config = GlobalConfig::load(None)?.unwrap_or_default();
            let active = config
                .active_context
                .clone()
                .unwrap_or_else(|| "default".to_string());

            let contexts: Vec<_> = config
                .project_contexts
                .iter()
                .map(|(name, ctx)| {
                    serde_json::json!({
                        "name": name,
                        "path": ctx.path.clone(),
                        "type": ctx.context_type.to_string(),
                        "active": name == &active
                    })
                })
                .collect();

            Ok::<_, toad_core::ToadError>(contexts)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(description = "Switch the active project context")]
    pub async fn switch_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SwitchContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = params.0.name;

        let result = tokio::task::spawn_blocking(move || {
            let mut config = GlobalConfig::load(None)?.unwrap_or_default();

            if !config.project_contexts.contains_key(&name) {
                return Err(toad_core::ToadError::ContextNotFound(name));
            }

            config.active_context = Some(name.clone());
            config.save(None)?;

            Ok::<_, toad_core::ToadError>(format!("Switched to context '{}'", name))
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

const INSTRUCTIONS: &str = "Toad is an AI-native ecosystem context oracle. \
It provides tools to query project metadata, search projects semantically, \
and retrieve high-fidelity architectural context across multiple repositories.";

#[async_trait]
#[tool_handler]
impl ServerHandler for ToadService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "toad-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                title: Some("Toad MCP Server".into()),
                website_url: Some("https://github.com/Primatif/Primatif_Toad".into()),
            },
            instructions: Some(INSTRUCTIONS.into()),
        }
    }
}
