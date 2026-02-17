use rmcp::model::{ErrorCode, ErrorData as McpError};
use toad_core::ToadError;

pub fn toad_error_to_mcp(err: ToadError) -> McpError {
    match err {
        ToadError::PathNotFound(path) => {
            McpError::resource_not_found(format!("Path not found: {:?}", path), None)
        }
        ToadError::ContextNotFound(name) => McpError::new(
            ErrorCode(-32004),
            format!("Context '{}' not found", name),
            None,
        ),
        ToadError::Io(e) => McpError::new(ErrorCode(-32002), e, None),
        ToadError::Serde(e) => McpError::new(ErrorCode(-32003), e, None),
        ToadError::WorkspaceNotFound => {
            McpError::new(ErrorCode(-32001), "Workspace not found".to_string(), None)
        }
        ToadError::Other(msg) => McpError::new(ErrorCode(-32000), msg, None),
        ToadError::Anyhow(msg) => McpError::new(ErrorCode(-32000), msg, None),
        _ => McpError::new(ErrorCode(-32000), err.to_string(), None),
    }
}
