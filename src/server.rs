use async_trait::async_trait;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{
    CallToolResult, ErrorData as McpError, Implementation, ProtocolVersion, ServerCapabilities,
    ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toad_core::Workspace;

use crate::tools;

#[derive(Clone)]
pub struct ToadService {
    pub tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct GetProjectDetailParams {
    /// Exact project name
    pub name: String,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct SearchProjectsParams {
    /// Search term
    pub query: String,
    /// Narrow search by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RevealParams {
    /// Search term
    pub query: String,
    /// Narrow search by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetEcosystemSummaryParams {
    /// Max tokens (default from config)
    pub token_limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetEcosystemStatusParams {
    /// Filter by project name (substring)
    pub query: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetProjectStatsParams {
    /// Filter by project name (substring)
    pub query: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct SwitchContextParams {
    /// Name of the context to switch to
    pub name: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[allow(dead_code)]
pub struct NoParams {
    // Empty
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct CompareProjectsParams {
    /// Source project name
    pub source: String,
    /// Target project name
    pub target: String,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct StatusParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct StatsParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
    /// Show details for all projects
    pub all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct BranchesParams {
    /// Optional query to filter projects
    pub query: Option<String>,
    /// Optional tag filter
    pub tag: Option<String>,
    /// Show remote branches
    pub all: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ManifestParams {
    /// Optional project name for project-specific context
    pub project: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RegisterContextParams {
    /// Context name
    pub name: String,
    /// Absolute path to projects directory
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
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

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeDepsParams {
    /// Optional query to filter projects
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeVelocityParams {
    /// Number of days to analyze (default: 30)
    pub days: Option<u32>,
    /// Optional query to filter projects
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeDebtParams {
    /// Optional query to filter projects
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeHealthParams {
    /// Optional query to filter projects
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct AnalyzeTrendsParams {
    /// Number of days to analyze (default: 90)
    pub days: Option<u32>,
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
        tools::discovery::list_projects(params).await
    }

    #[tool(
        description = "[Context] Get full metadata for a project including path, stack, submodules, and CONTEXT.md. Requires exact project name."
    )]
    pub async fn get_project_detail(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::get_project_detail(params).await
    }

    #[tool(
        description = "[Context] Get structural DNA patterns for a project (roles, capabilities). Use this to understand architectural patterns."
    )]
    pub async fn get_project_dna(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::get_project_dna(params).await
    }

    #[tool(
        description = "[Analysis] Compare two projects for migration compatibility. Returns compatibility score and migration recommendations."
    )]
    pub async fn compare_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<CompareProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::compare_projects(params).await
    }

    #[tool(
        description = "[Discovery] Search projects by DNA characteristics (role, capability, structural pattern). Find projects with specific patterns like 'async', 'REST API'."
    )]
    pub async fn search_projects_by_dna(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::search_projects_by_dna(params).await
    }

    #[tool(
        description = "[Discovery] Semantic search across project names, essence (README), and tags. Returns ranked results."
    )]
    pub async fn search_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchProjectsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::search_projects(params).await
    }

    #[tool(
        description = "[Discovery] Get high-level ecosystem summary (SYSTEM_PROMPT.md format). Token-limited overview of all projects."
    )]
    pub async fn get_ecosystem_summary(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetEcosystemSummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::get_ecosystem_summary(params).await
    }

    #[tool(
        description = "[Discovery] Get ecosystem health status showing VCS state and activity distribution. Identify projects needing attention."
    )]
    pub async fn get_ecosystem_status(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetEcosystemStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::get_ecosystem_status(params).await
    }

    #[tool(description = "[Analysis] Get project disk usage stats and bloat analytics.")]
    pub async fn get_project_stats(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectStatsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::get_project_stats(params).await
    }

    #[tool(description = "[Management] Get the currently active project context (Hub or Pond).")]
    pub async fn get_active_context(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::get_active_context().await
    }

    #[tool(description = "[Management] List all registered project contexts.")]
    pub async fn list_contexts(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::list_contexts().await
    }

    #[tool(
        description = "[Management] Switch the active project context (changes workspace root)."
    )]
    pub async fn switch_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SwitchContextParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::switch_context(params).await
    }

    #[tool(description = "[Context] Get the full architectural atlas (DNA map for all projects).")]
    pub async fn get_atlas(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::get_atlas().await
    }

    #[tool(description = "[Context] Get the full ecosystem manifest (detailed project list).")]
    pub async fn get_manifest(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::get_manifest().await
    }

    #[tool(
        description = "[Context] Get detailed context for a specific project (from its CONTEXT.md)."
    )]
    pub async fn get_project_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<GetProjectDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::get_project_context(params).await
    }

    #[tool(
        description = "[Discovery] Search for projects matching a query. Returns project names, paths, and basic metadata. Follow up with get_project_detail."
    )]
    pub async fn reveal_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<RevealParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::reveal_projects(params).await
    }

    #[tool(
        description = "[Discovery] Get Git status across all projects. Shows uncommitted changes, unpushed commits, and branch info."
    )]
    pub async fn get_git_status(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<StatusParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::get_git_status(params).await
    }

    #[tool(
        description = "[Analysis] Get disk usage analytics for projects. Shows total size, build artifacts, and bloat analysis. Note: Does NOT delete anything."
    )]
    pub async fn get_disk_stats(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<StatsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::get_disk_stats(params).await
    }

    #[tool(
        description = "[Discovery] List all branches across projects. Shows current branch and available local/remote branches."
    )]
    pub async fn list_branches(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<BranchesParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::discovery::list_branches(params).await
    }

    #[tool(
        description = "[Management] Rebuild the project registry cache by scanning the workspace. Run this after adding/removing projects."
    )]
    pub async fn sync_registry(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::sync_registry().await
    }

    #[tool(
        description = "[Context] Generate AI context files (MANIFEST.md, ATLAS.json, SYSTEM_PROMPT.md, CONTEXT.md). Refreshes AI intuition."
    )]
    pub async fn generate_manifest(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::context::generate_manifest(params).await
    }

    #[tool(
        description = "[Management] Register a new project context. Creates a new workspace configuration for a projects directory."
    )]
    pub async fn register_context(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<RegisterContextParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::register_context(params).await
    }

    #[tool(
        description = "[Management] Assign a tag to projects. Use query/tag filters to target specific projects. Harvest mode auto-detects stack tags."
    )]
    pub async fn tag_projects(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<TagParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::management::tag_projects(params).await
    }

    #[tool(
        description = "[Analysis] Run comprehensive health check on Toad installation and workspace. Returns categorized diagnostics and actionable recommendations."
    )]
    pub async fn run_health_check(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::run_health_check().await
    }

    #[tool(
        description = "[Analysis] Analyze project dependencies and critical path. Shows which projects depend on which crates, identifies critical path, and detects orphans."
    )]
    pub async fn analyze_dependencies(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeDepsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_dependencies(params).await
    }

    #[tool(
        description = "[Analysis] Get development velocity metrics for last N days. Shows commits per project, lines changed, and active contributors."
    )]
    pub async fn analyze_velocity(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeVelocityParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_velocity(params).await
    }

    #[tool(
        description = "[Analysis] Identify technical debt indicators. Shows TODO/FIXME counts, large files, and calculates a debt score."
    )]
    pub async fn analyze_debt(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeDebtParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_debt(params).await
    }

    #[tool(
        description = "[Analysis] Calculate composite project health scores (0-100). Includes AI-readiness metrics and proactive recommendations."
    )]
    pub async fn analyze_health(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeHealthParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_health(params).await
    }

    #[tool(
        description = "[Analysis] Analyze historical trends over time. Shows health score changes, disk usage trends, and activity trends."
    )]
    pub async fn analyze_trends(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeTrendsParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_trends(params).await
    }

    #[tool(
        description = "[Analysis] Analyze cross-project patterns and architectural consistency."
    )]
    pub async fn analyze_patterns(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_patterns().await
    }

    #[tool(description = "[Analysis] Analyze submodule health and alignment across the ecosystem.")]
    pub async fn analyze_submodules(
        &self,
        _params: rmcp::handler::server::wrapper::Parameters<NoParams>,
    ) -> Result<CallToolResult, McpError> {
        tools::analysis::analyze_submodules().await
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
                name: "toad-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some("Toad MCP Server - AI-native ecosystem context oracle".to_string()),
            },
            instructions: Some(INSTRUCTIONS.into()),
        }
    }
}
