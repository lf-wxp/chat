
#[cfg(feature = "dev")]
pub const SDP_SERVER: &str = "ws://127.0.0.1:8888";

#[cfg(not(feature = "dev"))]
pub const SDP_SERVER: &str = "ws://api.example.com/production";

