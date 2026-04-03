//! Statistics and monitoring API handlers
//!
//! Provides HTTP endpoints for:
//! - Filter statistics (sensitive word filtering metrics)
//! - Network quality statistics
//! - Audit log access

use axum::{
  Json,
  extract::{Path, Query, State},
  http::StatusCode,
  response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// =============================================================================
// Query Parameters
// =============================================================================

/// Query parameters for recent events
#[derive(Debug, Deserialize)]
pub struct RecentEventsQuery {
  /// Filter by user ID
  pub user_id: Option<String>,
  /// Filter by room ID
  pub room_id: Option<String>,
  /// Maximum number of events to return (default: 20, max: 100)
  #[serde(default = "default_limit")]
  pub limit: usize,
}

fn default_limit() -> usize {
  20
}

/// Query parameters for top words/users
#[derive(Debug, Deserialize)]
pub struct TopQuery {
  /// Maximum number of items to return (default: 10, max: 50)
  #[serde(default = "default_top_limit")]
  pub limit: usize,
}

fn default_top_limit() -> usize {
  10
}

// =============================================================================
// Response Types
// =============================================================================

/// Generic API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
  /// Whether the request was successful
  pub success: bool,
  /// Response data
  pub data: Option<T>,
  /// Error message (if any)
  pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
  /// Create a successful response
  pub fn success(data: T) -> Self {
    Self {
      success: true,
      data: Some(data),
      error: None,
    }
  }

  /// Create an error response
  pub fn error(message: impl Into<String>) -> Self {
    Self {
      success: false,
      data: None,
      error: Some(message.into()),
    }
  }
}

// =============================================================================
// Filter Statistics Endpoints
// =============================================================================

/// GET /api/stats/filter - Get overall filter statistics
pub async fn get_filter_stats(
  State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
  let stats = state.inner().filter_stats.get_statistics();
  Ok(Json(ApiResponse::success(stats)))
}

/// GET /api/stats/filter/recent - Get recent filter events
pub async fn get_recent_filter_events(
  State(state): State<AppState>,
  Query(query): Query<RecentEventsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
  let limit = query.limit.min(100);
  let events = state.inner().filter_stats.get_recent_events(
    query.user_id.as_deref(),
    query.room_id.as_deref(),
    limit,
  );
  Ok(Json(ApiResponse::success(events)))
}

/// GET /api/stats/filter/top-words - Get top filtered words
pub async fn get_top_filtered_words(
  State(state): State<AppState>,
  Query(query): Query<TopQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
  let limit = query.limit.min(50);
  let top_words = state.inner().filter_stats.get_top_words(limit);
  Ok(Json(ApiResponse::success(top_words)))
}

/// GET /api/stats/filter/top-users - Get top users by filter events
pub async fn get_top_filter_users(
  State(state): State<AppState>,
  Query(query): Query<TopQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
  let limit = query.limit.min(50);
  let top_users = state.inner().filter_stats.get_top_users(limit);
  Ok(Json(ApiResponse::success(top_users)))
}

/// GET /api/stats/filter/word/:word - Get statistics for a specific word
pub async fn get_word_stats(
  State(state): State<AppState>,
  Path(word): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
  match state.inner().filter_stats.get_word_stats(&word) {
    Some(stats) => Ok(Json(ApiResponse::success(stats))),
    None => Err((
      StatusCode::NOT_FOUND,
      Json(ApiResponse::error(format!(
        "Word '{}' not found in statistics",
        word
      ))),
    )),
  }
}

// =============================================================================
// Health Check
// =============================================================================

/// GET /api/stats/health - Health check for statistics service
pub async fn stats_health_check() -> impl IntoResponse {
  Json(ApiResponse::success(serde_json::json!({
    "status": "healthy",
    "service": "statistics"
  })))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_limit() {
    assert_eq!(default_limit(), 20);
  }

  #[test]
  fn test_default_top_limit() {
    assert_eq!(default_top_limit(), 10);
  }

  #[test]
  fn test_api_response_success() {
    let response = ApiResponse::success("test data");
    assert!(response.success);
    assert_eq!(response.data, Some("test data"));
    assert!(response.error.is_none());
  }

  #[test]
  fn test_api_response_error() {
    let response: ApiResponse<String> = ApiResponse::error("test error");
    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(response.error, Some("test error".to_string()));
  }
}
