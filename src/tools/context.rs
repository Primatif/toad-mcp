use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use std::fs;
use toad_core::{GlobalConfig, Workspace};

use crate::errors::toad_error_to_mcp;
use crate::server::{GetProjectDetailParams, ManifestParams};

pub async fn get_project_detail(
    params: Parameters<GetProjectDetailParams>,
) -> Result<CallToolResult, McpError> {
    let name = params.0.name;

    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(ws.active_context.as_deref(), None)?;

        let project = registry
            .projects
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| toad_core::ToadError::Other(format!("Project '{}' not found", name)))?;

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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_project_dna(
    params: Parameters<GetProjectDetailParams>,
) -> Result<CallToolResult, McpError> {
    let name = params.0.name;

    let result = tokio::task::spawn_blocking(move || {
        let _ws = Workspace::discover()?;
        let registry = toad_core::ProjectRegistry::load(_ws.active_context.as_deref(), None)?;

        let project = registry
            .projects
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| toad_core::ToadError::Other(format!("Project '{}' not found", name)))?;

        Ok::<_, toad_core::ToadError>(serde_json::to_string_pretty(&project.dna)?)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_atlas() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let atlas_path = ws.atlas_path();

        if !atlas_path.exists() {
            return Err(toad_core::ToadError::Other(
                "ATLAS.json not found. Run 'toad manifest' or 'generate_manifest' to generate it."
                    .to_string(),
            ));
        }

        let content = std::fs::read_to_string(atlas_path)?;
        Ok::<_, toad_core::ToadError>(content)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_manifest() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let manifest_path = ws.manifest_path();

        if !manifest_path.exists() {
            return Err(toad_core::ToadError::Other(
                "MANIFEST.md not found. Run 'toad manifest' or 'generate_manifest' to generate it."
                    .to_string(),
            ));
        }

        let content = std::fs::read_to_string(manifest_path)?;
        Ok::<_, toad_core::ToadError>(content)
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn get_project_context(
    params: Parameters<GetProjectDetailParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn generate_manifest(
    params: Parameters<ManifestParams>,
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
        let system_prompt =
            toad_manifest::generate_system_prompt(&projects, Some(config.budget.ecosystem_tokens));
        fs::write(ws.shadows_dir.join("SYSTEM_PROMPT.md"), system_prompt)?;

        // llms.txt
        let llms_txt = toad_manifest::generate_llms_txt(&projects);
        fs::write(ws.shadows_dir.join("llms.txt"), llms_txt)?;

        // Per-project
        for p in &projects {
            let proj_shadow_dir = ws.shadows_dir.join(&p.name);
            fs::create_dir_all(&proj_shadow_dir)?;

            let context_md =
                toad_manifest::generate_project_context_md(p, Some(config.budget.project_tokens));
            fs::write(proj_shadow_dir.join("CONTEXT.md"), context_md)?;
        }

        Ok::<_, toad_core::ToadError>(format!(
            "Manifest and tiered prompts generated for {} projects",
            projects.len()
        ))
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
