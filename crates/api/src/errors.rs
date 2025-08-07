// https://github.com/LemmyNet/lemmy/blob/main/crates/utils/src/error.rs#L73
use axum::{extract::rejection::JsonRejection, http::StatusCode};
use serde_json::json;
use std::fmt::{Debug, Display};
use tracing_error::SpanTrace;

#[allow(dead_code)]
pub type SrvResult<T> = Result<T, SonarError>;

// https://docs.rs/tracing-error/latest/tracing_error/
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum SonarErrorKind {
    // The request body contained invalid JSON
    #[error("{0}")]
    JsonRejection(#[from] JsonRejection),

    #[error("{0}")]
    ValidationError(#[from] validator::ValidationErrors),

    #[error("the data for key {0} is not found")]
    NotFound(String),

    #[error("{1}")]
    Custom(StatusCode, String),

    #[error("{0}")]
    Any(#[from] anyhow::Error),

    #[error("storage error: `{0}`")]
    StorageError(#[from] sonar_db::StorageError),

    #[error("invalid query: `{0}`")]
    InvalidQuery(String),

    #[error("invalid json: `{0}`")]
    InvalidJson(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct SonarError {
    pub error_kind: SonarErrorKind,
    pub inner: anyhow::Error,
    pub context: SpanTrace,
}

impl Display for SonarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: ", &self.error_kind)?;
        // print anyhow including trace
        // https://docs.rs/anyhow/latest/anyhow/struct.Error.html#display-representations
        // this will print the anyhow trace (only if it exists)
        // and if RUST_BACKTRACE=1, also a full backtrace
        writeln!(f, "{:?}", self.inner)?;
        // writeln!(f, "source {:?}", self.inner.backtrace())?;
        // print the tracing span trace
        std::fmt::Display::fmt(&self.context, f)
    }
}

impl<T> From<T> for SonarError
where
    T: Into<SonarErrorKind>,
{
    fn from(t: T) -> Self {
        let into = t.into();
        SonarError {
            inner: anyhow::anyhow!("{:?}", &into),
            error_kind: into,
            context: SpanTrace::capture(),
        }
    }
}

impl axum::response::IntoResponse for SonarError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self.error_kind {
            SonarErrorKind::JsonRejection(_) => StatusCode::BAD_REQUEST,
            SonarErrorKind::InvalidQuery(_) => StatusCode::BAD_REQUEST,
            SonarErrorKind::Custom(code, _) => code,
            SonarErrorKind::ValidationError(_) => StatusCode::BAD_REQUEST,
            SonarErrorKind::NotFound(_) => StatusCode::NOT_FOUND,
            SonarErrorKind::Any(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SonarErrorKind::StorageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = axum::Json(json!({
            "success": false,
            "code": status_code.as_u16(),
            "error": status_code.canonical_reason().unwrap_or("Unknown").to_string(),
            "message": self.error_kind.to_string(),
        }));
        (status_code, body).into_response()
    }
}
