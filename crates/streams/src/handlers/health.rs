use axum::response::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Handler to get the liveness of the service
pub async fn get_health() -> Json<HealthResponse> {
    let response =
        HealthResponse { status: "ok".to_string(), version: env!("CARGO_PKG_VERSION").to_string() };
    Json(response)
}
