use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

use crate::common::di::AppState;

// Temporary placeholder implementation
pub fn caldav_routes() -> Router<AppState> {
    Router::new().route("/placeholder", get(placeholder_handler))
}

async fn placeholder_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "message": "CalDAV functionality is not yet implemented"
        })),
    )
}
