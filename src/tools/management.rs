use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData as McpError};
use std::fs;
use toad_core::{GlobalConfig, Workspace};

use crate::errors::toad_error_to_mcp;
use crate::server::{RegisterContextParams, SwitchContextParams, TagParams};

pub async fn get_active_context() -> Result<CallToolResult, McpError> {
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub async fn list_contexts() -> Result<CallToolResult, McpError> {
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )]))
}

pub async fn switch_context(
    params: Parameters<SwitchContextParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn sync_registry() -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let ws = Workspace::discover()?;
        let reporter = toad_core::NoOpReporter;
        let count = toad_discovery::sync_registry(&ws, &reporter)?;
        Ok::<_, toad_core::ToadError>(format!("Registry synchronized ({} projects found)", count))
    })
    .await
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn register_context(
    params: Parameters<RegisterContextParams>,
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

pub async fn tag_projects(params: Parameters<TagParams>) -> Result<CallToolResult, McpError> {
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
    .map_err(|e| toad_error_to_mcp(toad_core::ToadError::Other(e.to_string())))?
    .map_err(toad_error_to_mcp)?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
