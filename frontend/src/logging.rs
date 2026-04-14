//! Frontend logging module.

use tracing::info;

/// Initialize frontend logging.
pub(crate) fn init() {
  // Console logging is configured automatically by tracing-wasm
  info!("Frontend logging initialized");
}
