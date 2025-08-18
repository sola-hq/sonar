use axum::response::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, PartialEq, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Handler to get the liveness of the service
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn get_health() -> Json<HealthResponse> {
    let response =
        HealthResponse { status: "ok".to_string(), version: env!("CARGO_PKG_VERSION").to_string() };
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_get_health() {
        let app = Router::new().route("/health", get(get_health));
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .expect("Failed to call health endpoint");

        assert!(response.status().is_success());
    }
}
