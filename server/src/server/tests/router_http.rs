use super::*;
use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn test_router_ws_route_exists() {
  let config = Config::default();
  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  // Send a plain GET to /ws (without WebSocket upgrade headers).
  // The server should respond with a non-404 status, proving the route exists.
  // Without proper upgrade headers, axum returns 400 or similar, NOT 404.
  let request = axum::http::Request::builder()
    .uri("/ws")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  // A registered route without proper upgrade headers should NOT return 404
  assert_ne!(
    response.status().as_u16(),
    404,
    "The /ws route should be registered"
  );
}

#[tokio::test]
async fn test_router_unknown_asset_returns_404() {
  // Paths that look like asset requests (have a file extension in the
  // last segment) must remain an honest 404 when the file is missing,
  // so broken asset links surface loudly in tooling instead of being
  // silently swallowed by the SPA fallback.
  let config = Config::default();
  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  let request = axum::http::Request::builder()
    .uri("/does-not-exist.png")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    404,
    "Non-existent asset path (with file extension) should return 404"
  );
}

#[tokio::test]
async fn test_router_spa_deep_link_serves_index_html() {
  // Navigation-style deep-link URLs (no file extension) must fall back
  // to the SPA shell so the frontend router can handle them after a
  // hard refresh. This is the contract that keeps PWA `start_url`
  // deep links working.
  let temp_dir = std::env::temp_dir().join("server_test_spa_fallback");
  std::fs::create_dir_all(&temp_dir).unwrap();
  std::fs::write(temp_dir.join("index.html"), "<html>spa-shell</html>").unwrap();

  let config = Config {
    static_dir: temp_dir.clone(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  let request = axum::http::Request::builder()
    .uri("/room/some-uuid")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "SPA deep link should be answered by the index.html shell"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  assert_eq!(body.as_ref(), b"<html>spa-shell</html>");

  // Cleanup
  let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_static_file_serving() {
  // Create a temporary directory with a test file to verify static serving
  let temp_dir = std::env::temp_dir().join("server_test_static");
  std::fs::create_dir_all(&temp_dir).unwrap();
  std::fs::write(temp_dir.join("test.txt"), "hello from static").unwrap();

  let config = Config {
    static_dir: temp_dir.clone(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  let request = axum::http::Request::builder()
    .uri("/test.txt")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "Static file should be served"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  assert_eq!(body.as_ref(), b"hello from static");

  // Cleanup
  let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_index_html_on_directory() {
  // Create a temporary directory with index.html to verify directory index serving
  let temp_dir = std::env::temp_dir().join("server_test_index");
  std::fs::create_dir_all(&temp_dir).unwrap();
  std::fs::write(temp_dir.join("index.html"), "<html>index</html>").unwrap();

  let config = Config {
    static_dir: temp_dir.clone(),
    ..Default::default()
  };

  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  // Request the root path — should serve index.html
  let request = axum::http::Request::builder()
    .uri("/")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "Root path should serve index.html"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  assert_eq!(body.as_ref(), b"<html>index</html>");

  // Cleanup
  let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_router_health_endpoint_returns_200() {
  // The /api/health liveness probe is consumed by Docker / Kubernetes
  // healthchecks (see Dockerfile + docker-compose.yml). It must stay
  // registered and respond 200 with a JSON body so those integrations
  // stay green.
  let config = Config::default();
  let server = Server::new(config);
  let (router, _ws_state) = server.build_router();

  let request = axum::http::Request::builder()
    .uri("/api/health")
    .method("GET")
    .body(Body::empty())
    .unwrap();

  let response = router.oneshot(request).await.unwrap();
  assert_eq!(
    response.status().as_u16(),
    200,
    "Health endpoint must respond 200 OK"
  );

  let body = response.into_body().collect().await.unwrap().to_bytes();
  let text = std::str::from_utf8(&body).unwrap();
  assert!(text.contains("\"status\":\"ok\""), "body was: {text}");
  assert!(
    text.contains("\"service\":\"webrtc-chat-signaling\""),
    "body was: {text}"
  );
}

#[tokio::test]
async fn test_server_start_fails_on_invalid_address() {
  // Bind to a port that is already in use to trigger a start failure.
  // First, occupy a port:
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let occupied_port = listener.local_addr().unwrap().port();

  // Try to start a server on the same port
  let config = Config {
    addr: std::net::SocketAddr::from(([127, 0, 0, 1], occupied_port)),
    ..Default::default()
  };

  let server = Server::new(config);
  let result = server.start().await;

  // Should fail because the port is already occupied
  assert!(
    result.is_err(),
    "Server should fail to start on occupied port"
  );
}
