//! Browser notifications for incoming calls (Req 7.4).
//!
//! Thin wrapper that delegates to the unified `crate::notifications`
//! module so the user-level "Call Notifications" toggle and the
//! Do-Not-Disturb window can gate every popup in one place
//! (Req 13.4.3 / 13.4.4).

/// Show a browser notification for an incoming call.
///
/// Honours the "Call Notifications" toggle and the Do-Not-Disturb
/// window persisted in the user-settings store. See
/// [`crate::notifications::show_notification`] for the full gate
/// semantics.
pub fn show_incoming_call_notification(title: String, body: String) {
  crate::notifications::show_incoming_call_notification(title, body);
}
