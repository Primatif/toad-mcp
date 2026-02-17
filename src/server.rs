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
pub struct RevealParams {
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

#[derive(Deserialize, JsonSchema)]
pub struct StatusParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct StatsParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
    /// Show details for all projects
    #[allow(dead_code)]
    pub all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct BranchesParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
    /// Show remote branches
    pub all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ManifestParams {
    /// Optional project name for project-specific context
    pub project: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RegisterContextParams {
    /// Context name
    pub name: String,
    /// Absolute path to projects directory
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TagParams {
    /// Project name (optional if using filters)
    pub project: Option<String>,
    /// Tag name
    pub tag: Option<String>,
    /// Filter by name query
    pub query: Option<String>,
    /// Filter by existing tag
    pub filter_tag: Option<String>,
    /// Auto-harvest stack tags
    pub harvest: Option<bool>,
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

    #[tool(
        description = "[Discovery] List projects with optional filters. Returns basic metadata. Use get_project_detail for full info."
    )]
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

    #[tool(
        description = "[Context] Get full metadata for a project including path, stack, submodules, and CONTEXT.md. Requires exact project name."
    )]
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

    #[tool(
        description = "[Context] Get structural DNA patterns for a project (roles, capabilities). Use this to understand architectural patterns."
    )]
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

    #[tool(
        description = "[Analysis] Compare two projects for migration compatibility. Returns compatibility score and migration recommendations."
    )]
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

    #[tool(
        description = "[Discovery] Search projects by DNA characteristics (role, capability, structural pattern). Find projects with specific patterns like 'async', 'REST API'."
    )]
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

    #[tool(
        description = "[Discovery] Semantic search across project names, essence (README), and tags. Returns ranked results."
    )]
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

    #[tool(
        description = "[Discovery] Get high-level ecosystem summary (SYSTEM_PROMPT.md format). Token-limited overview of all projects."
    )]
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

    #[tool(
        description = "[Discovery] Get ecosystem health status showing VCS state and activity distribution. Identify projects needing attention."
    )]
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

    #[tool(description = "[Analysis] Get project disk usage stats and bloat analytics.")]
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

    #[tool(description = "[Management] Get the currently active project context (Hub or Pond).")]
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

    #[tool(description = "[Management] List all registered project contexts.")]
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

    #[tool(
        description = "[Management] Switch the active project context (changes workspace root)."
    )]
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

    #[tool(description = "[Context] Get the full architectural atlas (DNA map for all projects).")]
    pub async fn get_atlas(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let atlas_path = ws.atlas_path();

            if !atlas_path.exists() {
                return Err(toad_core::ToadError::Other(
                    "ATLAS.json not found. Run 'toad manifest' or 'generate_manifest' to generate it.".to_string(),
                ));
            }

            let content = std::fs::read_to_string(atlas_path)?;
            Ok::<_, toad_core::ToadError>(content)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "[Context] Get the full ecosystem manifest (detailed project list).")]
    pub async fn get_manifest(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let manifest_path = ws.manifest_path();

            if !manifest_path.exists() {
                return Err(toad_core::ToadError::Other(
                    "MANIFEST.md not found. Run 'toad manifest' or 'generate_manifest' to generate it.".to_string(),
                ));
            }

            let content = std::fs::read_to_string(manifest_path)?;
            Ok::<_, toad_core::ToadError>(content)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Context] Get detailed context for a specific project (from its CONTEXT.md)."
    )]
    pub async fn get_project_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = params.0.name;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;

            let context_path = ws.shadows_dir.join(&name).join("CONTEXT.md");

            if !context_path.exists() {
                return Err(toad_core::ToadError::Other(format!(
                    "CONTEXT.md for project '{}' not found. Run 'toad manifest' or 'generate_manifest' to generate it.",
                    name
                )));
            }

            let content = std::fs::read_to_string(context_path)?;
            Ok::<_, toad_core::ToadError>(content)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Discovery] Search for projects matching a query. Returns project names, paths, and basic metadata. Follow up with get_project_detail."
    )]
    pub async fn reveal_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<RevealParams>,
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

    #[tool(
        description = "[Discovery] Get Git status across all projects. Shows uncommitted changes, unpushed commits, and branch info."
    )]
    pub async fn get_git_status(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query;
        let tag = params.0.tag;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let targets: Vec<_> = registry
                .projects
                .into_iter()
                .filter(|p| {
                    if let Some(q) = &query
                        && !p.name.to_lowercase().contains(&q.to_lowercase())
                    {
                        return false;
                    }
                    if let Some(t) = &tag
                        && !p
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase() == t.to_lowercase())
                    {
                        return false;
                    }
                    true
                })
                .collect();

            if targets.is_empty() {
                return Ok::<_, toad_core::ToadError>(
                    "No projects found matching filters.".to_string(),
                );
            }

            let report = toad_git::generate_multi_repo_status(&targets)?;
            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Analysis] Get disk usage analytics for projects. Shows total size, build artifacts, and bloat analysis. Note: Does NOT delete anything."
    )]
    pub async fn get_disk_stats(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<StatsParams>,
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

    #[tool(
        description = "[Discovery] List all branches across projects. Shows current branch and available local/remote branches."
    )]
    pub async fn list_branches(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<BranchesParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.0.query;
        let tag = params.0.tag;
        let all = params.0.all.unwrap_or(false);

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

            let targets: Vec<_> = registry
                .projects
                .into_iter()
                .filter(|p| {
                    if let Some(q) = &query
                        && !p.name.to_lowercase().contains(&q.to_lowercase())
                    {
                        return false;
                    }
                    if let Some(t) = &tag
                        && !p
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase() == t.to_lowercase())
                    {
                        return false;
                    }
                    true
                })
                .collect();

            if targets.is_empty() {
                return Ok::<_, toad_core::ToadError>(
                    "No projects found matching filters.".to_string(),
                );
            }

            let mut output = Vec::new();
            for p in targets {
                let local = toad_git::branches::list_local_branches(&p.path)?;
                let mut branches = local;
                if all {
                    let remote = toad_git::branches::list_remote_branches(&p.path)?;
                    branches.extend(remote);
                }

                output.push(serde_json::json!({
                    "project": p.name,
                    "branches": branches,
                }));
            }

            Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&output)?)
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Management] Rebuild the project registry cache by scanning the workspace. Run this after adding/removing projects."
    )]
    pub async fn sync_registry(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let reporter = toad_core::NoOpReporter;
            let count = toad_discovery::sync_registry(&ws, &reporter)?;
            Ok::<_, toad_core::ToadError>(format!(
                "Registry synchronized ({} projects found)",
                count
            ))
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Context] Generate AI context files (MANIFEST.md, ATLAS.json, SYSTEM_PROMPT.md, CONTEXT.md). Refreshes AI intuition."
    )]
    pub async fn generate_manifest(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        let project_filter = params.0.project;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let current_fp = ws.get_fingerprint()?;
            let config = GlobalConfig::load(None)?.unwrap_or_default();

            // 1. Sync first
            let reporter = toad_core::NoOpReporter;
            toad_discovery::sync_registry(&ws, &reporter)?;

            // 2. Load projects
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
            let projects: Vec<_> = registry
                .projects
                .iter()
                .filter(|p| {
                    if let Some(f) = &project_filter {
                        p.name.to_lowercase().contains(&f.to_lowercase())
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            if projects.is_empty() {
                return Err(toad_core::ToadError::Other(
                    "No projects found matching filter".to_string(),
                ));
            }

            // 3. Generate files
            ws.ensure_shadows()?;

            // MANIFEST.md
            let manifest_md = toad_manifest::generate_markdown(
                &projects,
                current_fp,
                Some(config.budget.ecosystem_tokens),
            );
            fs::write(ws.manifest_path(), manifest_md)?;

            // SYSTEM_PROMPT.md
            let system_prompt = toad_manifest::generate_system_prompt(
                &projects,
                Some(config.budget.ecosystem_tokens),
            );
            fs::write(ws.shadows_dir.join("SYSTEM_PROMPT.md"), system_prompt)?;

            // llms.txt
            let llms_txt = toad_manifest::generate_llms_txt(&projects);
            fs::write(ws.shadows_dir.join("llms.txt"), llms_txt)?;

            // Per-project
            for p in &projects {
                let proj_shadow_dir = ws.shadows_dir.join(&p.name);
                fs::create_dir_all(&proj_shadow_dir)?;

                let context_md = toad_manifest::generate_project_context_md(
                    p,
                    Some(config.budget.project_tokens),
                );
                fs::write(proj_shadow_dir.join("CONTEXT.md"), context_md)?;
            }

            Ok::<_, toad_core::ToadError>(format!(
                "Manifest and tiered prompts generated for {} projects",
                projects.len()
            ))
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Management] Register a new project context. Creates a new workspace configuration for a projects directory."
    )]
    pub async fn register_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<RegisterContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = params.0.name;
        let path = params.0.path;

        let result = tokio::task::spawn_blocking(move || {
            let mut config = GlobalConfig::load(None)?.unwrap_or_default();

            let abs_path = fs::canonicalize(std::path::PathBuf::from(&path))
                .map_err(|e| toad_core::ToadError::Other(format!("Invalid path: {}", e)))?;

            if !abs_path.exists() {
                return Err(toad_core::ToadError::Other(format!(
                    "Path does not exist: {:?}",
                    abs_path
                )));
            }

            if config.project_contexts.contains_key(&name) {
                return Err(toad_core::ToadError::Other(format!(
                    "Context '{}' already exists",
                    name
                )));
            }

            // Auto-detect type
            let detected_type = if abs_path.join(".gitmodules").exists() {
                toad_core::ContextType::Hub
            } else if abs_path.join("projects").exists() {
                toad_core::ContextType::Pond
            } else {
                toad_core::ContextType::Generic
            };

            let ctx = toad_core::ProjectContext {
                path: abs_path.clone(),
                description: None,
                context_type: detected_type,
                ai_vendors: Vec::new(),
                registered_at: std::time::SystemTime::now(),
            };

            config.project_contexts.insert(name.clone(), ctx);

            // Create per-context storage
            let ctx_shadows = GlobalConfig::context_dir(&name, None)
                .map_err(|e| toad_core::ToadError::Other(e.to_string()))?
                .join("shadows");
            fs::create_dir_all(&ctx_shadows)?;

            config.save(None)?;

            Ok::<_, toad_core::ToadError>(format!(
                "Context '{}' ({}) registered at {:?}",
                name, detected_type, abs_path
            ))
        })
        .await
        .map_err(|e| crate::errors::toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
        .map_err(crate::errors::toad_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(
        description = "[Management] Assign a tag to projects. Use query/tag filters to target specific projects. Harvest mode auto-detects stack tags."
    )]
    pub async fn tag_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<TagParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        let result = tokio::task::spawn_blocking(move || {
            let ws = Workspace::discover()?;
            let mut tag_reg = toad_core::TagRegistry::load(&ws.tags_path())?;
            let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
            let projects = registry.projects;

            let mut targets = Vec::new();

            if params.harvest.unwrap_or(false) {
                for p in projects {
                    let stack_tag = p.stack.to_lowercase();
                    tag_reg.add_tag(&p.name, &stack_tag);
                    targets.push(p.name.clone());

                    for sub in p.submodules {
                        let sub_stack_tag = sub.stack.to_lowercase();
                        tag_reg.add_tag(&sub.name, &sub_stack_tag);
                        targets.push(sub.name.clone());
                    }
                }
            } else if params.query.is_some() || params.filter_tag.is_some() {
                let t_name = params.tag.or(params.project).ok_or_else(|| {
                    toad_core::ToadError::Other("Must provide a tag name to assign.".to_string())
                })?;

                let matching: Vec<_> = projects
                    .into_iter()
                    .filter(|p| {
                        let name_match = match &params.query {
                            Some(q) => p.name.to_lowercase().contains(&q.to_lowercase()),
                            None => true,
                        };
                        let tag_match = match &params.filter_tag {
                            Some(t) => {
                                let target = if t.starts_with('#') {
                                    t.clone()
                                } else {
                                    format!("#{}", t)
                                };
                                p.tags
                                    .iter()
                                    .any(|tag| tag.to_lowercase() == target.to_lowercase())
                            }
                            None => true,
                        };
                        name_match && tag_match
                    })
                    .collect();

                if matching.is_empty() {
                    return Ok::<_, toad_core::ToadError>(
                        "No projects found matching filters.".to_string(),
                    );
                }

                for p in matching {
                    tag_reg.add_tag(&p.name, &t_name);
                    targets.push(p.name);
                }
            } else if let Some(p_name) = params.project {
                if let Some(t_name) = params.tag {
                    tag_reg.add_tag(&p_name, &t_name);
                    targets.push(p_name);
                } else {
                    return Err(toad_core::ToadError::Other(
                        "Must provide a tag name.".to_string(),
                    ));
                }
            } else {
                return Err(toad_core::ToadError::Other(
                    "Must provide a project name or use filters.".to_string(),
                ));
            }

            tag_reg.save(&ws.tags_path())?;
            Ok::<_, toad_core::ToadError>(format!("Tagged {} projects.", targets.len()))
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
