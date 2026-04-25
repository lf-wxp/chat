//! Global error handling service.
//!
//! Provides centralized error processing for `ErrorResponse` messages
//! received from the signaling server. Displays user-friendly i18n
//! error messages as toast notifications with optional "Learn more"
//! expandable details.

mod manager;

pub use manager::{
  ErrorToast, ErrorToastManager, provide_error_toast_manager, use_error_toast_manager,
};

#[cfg(test)]
pub(crate) use manager::{MAX_TOASTS, next_toast_id};

#[cfg(test)]
mod tests;
