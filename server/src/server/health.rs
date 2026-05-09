//! Health check endpoint and SPA fallback handler.
//!
//! The health endpoint is a lightweight liveness probe used by Docker,
//! Kubernetes, and load balancers to verify the server is responsive.
//!
//! The SPA fallback handler ensures that deep-link URLs (e.g. `/room/xyz`,
//! `/settings`) served from the Progressive Web App shell correctly load
//! the frontend `index.html` instead of returning a 404 from the static
//! file service.

use std::path::PathBuf;

use axum::Json;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::fs;

/// Health check response body.
///
/// Minimal, stable shape so external orchestrators can rely on it
/// without risking breaking changes.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
  /// Constant marker; always `"ok"` when the HTTP handler runs.
  pub status: &'static str,
  /// Service identifier — useful when multiple services run behind
  /// the same load balancer.
  pub service: &'static str,
  /// Build version taken from Cargo at compile time.
  pub version: &'static str,
}

/// Handle `GET /api/health`.
///
/// Returns a fixed JSON document so liveness probes never trigger
/// allocation-heavy paths. The 200 status code alone is the contract
/// external systems rely on; the body is informational.
pub async fn health_check() -> Json<HealthResponse> {
  Json(HealthResponse {
    status: "ok",
    service: "webrtc-chat-signaling",
    version: env!("CARGO_PKG_VERSION"),
  })
}

/// SPA fallback handler.
///
/// Serves the `index.html` shell for any GET request that does not
/// match an existing static file, so the frontend router can handle
/// deep-link URLs and PWA start URLs survive a full refresh. Non-GET
/// requests and explicit asset-looking paths (with a file extension)
/// still return a proper 404 instead of the HTML shell.
pub async fn spa_fallback(State(static_dir): State<PathBuf>, req: Request) -> Response {
  // Only fall back for navigational GET requests; mutating verbs or
  // requests for a specific file extension must remain honest 404s so
  // broken asset links surface loudly in tooling.
  if req.method() != axum::http::Method::GET {
    return not_found();
  }
  if has_file_extension(req.uri()) {
    return not_found();
  }

  let index_path = static_dir.join("index.html");
  match fs::read(&index_path).await {
    Ok(bytes) => {
      let mut response = Response::new(Body::from(bytes));
      response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
      );
      // PWA shell must always stay fresh so new releases propagate
      // on navigation; static assets below still carry their own
      // long-lived cache policy via `ServeDir`.
      response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
      response
    }
    Err(_) => not_found(),
  }
}

/// Return whether the request URI ends with a segment containing a
/// recognisable file extension. Used to avoid turning 404s for
/// missing assets into 200 HTML responses.
fn has_file_extension(uri: &Uri) -> bool {
  let path = uri.path();
  let last_segment = path.rsplit('/').next().unwrap_or("");
  last_segment.contains('.')
}

/// Generic 404 response used by the SPA fallback when it cannot or
/// should not serve the HTML shell.
fn not_found() -> Response {
  (StatusCode::NOT_FOUND, "Not Found").into_response()
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::http::Uri;

  #[test]
  fn detects_file_extension_on_last_segment() {
    let with_ext: Uri = "/assets/icon-192.png".parse().unwrap();
    let no_ext: Uri = "/room/abc-123".parse().unwrap();
    let root: Uri = "/".parse().unwrap();
    let dotted_dir: Uri = "/v1.2/about".parse().unwrap();

    assert!(has_file_extension(&with_ext));
    assert!(!has_file_extension(&no_ext));
    assert!(!has_file_extension(&root));
    assert!(!has_file_extension(&dotted_dir));
  }

  #[tokio::test]
  async fn health_check_returns_ok_status() {
    let Json(payload) = health_check().await;
    assert_eq!(payload.status, "ok");
    assert_eq!(payload.service, "webrtc-chat-signaling");
    assert!(!payload.version.is_empty());
  }
}
