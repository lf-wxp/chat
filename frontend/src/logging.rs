//! Frontend logging module.

use tracing::info;

/// Initialize frontend logging.
#[allow(unreachable_pub)]
pub fn init() {
  // Console logging is configured automatically by tracing-wasm
  info!("Frontend logging initialized");
}
