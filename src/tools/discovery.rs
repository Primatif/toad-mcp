use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use toad_core::Workspace;

use crate::errors::toad_error_to_mcp;
use crate::server::{
    GetEcosystemStatusParams, GetEcosystemSummaryParams, ListProjectsParams, RevealParams,
    SearchProjectsParams,
};

pub async fn list_projects(
    params: Parameters<ListProjectsParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn search_projects_by_dna(
    params: Parameters<SearchProjectsParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn search_projects(
    params: Parameters<SearchProjectsParams>,
) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let tag = params.0.tag;

    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let search_result = toad_discovery::search_projects(&ws, &query, tag.as_deref())?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&search_result)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_ecosystem_summary(
    params: Parameters<GetEcosystemSummaryParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_ecosystem_status(
    params: Parameters<GetEcosystemStatusParams>,
) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let tag = params.0.tag;

    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let report = toad_discovery::generate_status_report(&ws, query.as_deref(), tag.as_deref())?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn reveal_projects(params: Parameters<RevealParams>) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let tag = params.0.tag;

    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let search_result = toad_discovery::search_projects(&ws, &query, tag.as_deref())?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&search_result)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_git_status(
    params: Parameters<crate::server::StatusParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn list_branches(
    params: Parameters<crate::server::BranchesParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
