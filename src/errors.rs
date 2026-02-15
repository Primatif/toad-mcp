use rmcp::model::{ErrorCode, ErrorData};
use toad_core::ToadError;

pub fn toad_error_to_mcp(err: ToadError) -> ErrorData {
    let code = match &err {
        ToadError::WorkspaceNotFound => ErrorCode::INTERNAL_ERROR,
        ToadError::PathNotFound(_) => ErrorCode::INVALID_PARAMS,
        ToadError::ContextNotFound(_) => ErrorCode::INVALID_PARAMS,
        ToadError::Config(_) => ErrorCode::INTERNAL_ERROR,
        ToadError::Io(_) => ErrorCode::INTERNAL_ERROR,
        ToadError::Serde(_) => ErrorCode::INTERNAL_ERROR,
        ToadError::Git(_) => ErrorCode::INTERNAL_ERROR,
        _ => ErrorCode::INTERNAL_ERROR,
    };

    ErrorData {
        code,
        message: err.to_string().into(),
        data: None,
    }
}
