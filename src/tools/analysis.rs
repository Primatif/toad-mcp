use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use toad_core::Workspace;

use crate::errors::toad_error_to_mcp;
use crate::server::{
    AnalyzeDebtParams, AnalyzeDepsParams, AnalyzeHealthParams, AnalyzeTrendsParams,
    AnalyzeVelocityParams, CompareProjectsParams, StatsParams,
};

pub async fn compare_projects(
    params: Parameters<CompareProjectsParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_project_stats(
    params: Parameters<crate::server::GetProjectStatsParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_disk_stats(params: Parameters<StatsParams>) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let tag = params.0.tag;
    let all = params.0.all.unwrap_or(false);

    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

        // Filter projects based on query and tag
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

        // If all is false, we might want to group or summarize, but generate_analytics_report
        // currently takes a slice and filters.
        // For now, we'll just use the standard report on filtered targets.
        let report = toad_ops::stats::generate_analytics_report(
            &targets, None, // already filtered
            None, // already filtered
        );

        let mut val = serde_json::to_value(report)?;
        if !all {
            // If not 'all', remove individual project details to save tokens
            if let Some(obj) = val.as_object_mut() {
                obj.remove("projects");
            }
        }

        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&val)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn run_health_check() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let report = toad_ops::doctor::run_health_check(&ws)?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_dependencies(
    params: Parameters<AnalyzeDepsParams>,
) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let targets: Vec<_> = registry
            .projects
            .into_iter()
            .filter(|p| {
                query
                    .as_ref()
                    .is_none_or(|q| p.name.to_lowercase().contains(&q.to_lowercase()))
            })
            .collect();
        let graph = toad_ops::analytics::analyze_dependencies(&targets)?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&graph)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_velocity(
    params: Parameters<AnalyzeVelocityParams>,
) -> Result<CallToolResult, McpError> {
    let days = params.0.days.unwrap_or(30);
    let query = params.0.query;
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let mut results = std::collections::HashMap::new();
        for p in registry.projects {
            if query
                .as_ref()
                .is_none_or(|q| p.name.to_lowercase().contains(&q.to_lowercase()))
            {
                let velocity = toad_ops::analytics::analyze_velocity(&p.path, days)?;
                results.insert(p.name, velocity);
            }
        }
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&results)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_debt(
    params: Parameters<AnalyzeDebtParams>,
) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let mut results = std::collections::HashMap::new();
        for p in registry.projects {
            if query
                .as_ref()
                .is_none_or(|q| p.name.to_lowercase().contains(&q.to_lowercase()))
            {
                let debt = toad_ops::analytics::analyze_debt(&p.path)?;
                results.insert(p.name, debt);
            }
        }
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&results)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_health(
    params: Parameters<AnalyzeHealthParams>,
) -> Result<CallToolResult, McpError> {
    let query = params.0.query;
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let mut results = std::collections::HashMap::new();
        for p in registry.projects {
            if query
                .as_ref()
                .is_none_or(|q| p.name.to_lowercase().contains(&q.to_lowercase()))
            {
                let health = toad_ops::analytics::calculate_health_score(&p)?;
                results.insert(p.name, health);
            }
        }
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&results)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_trends(
    params: Parameters<AnalyzeTrendsParams>,
) -> Result<CallToolResult, McpError> {
    let days = params.0.days.unwrap_or(90);
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let report = toad_ops::analytics::analyze_trends(&ws.projects_dir, days)?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&report)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_patterns() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let metrics = toad_ops::analytics::analyze_patterns(&registry.projects)?;
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&metrics)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn analyze_submodules() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;
        let mut results = Vec::new();
        for p in registry.projects {
            for sub in p.submodules {
                results.push(serde_json::json!({
                    "project": p.name,
                    "submodule": sub.name,
                    "initialized": sub.initialized,
                    "vcs_status": sub.vcs_status,
                }));
            }
        }
        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&results)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}
