
#[cfg(feature = "dev")]
pub const SDP_SERVER: &str = "http://127.0.0.1:8888";

#[cfg(not(feature = "dev"))]
pub const SDP_SERVER: &str = "https://api.example.com/production";

