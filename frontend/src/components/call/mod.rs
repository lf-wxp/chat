//! Call-UI components.
//!
//! Every sub-module here defines exactly one Leptos component (per
//! project convention). The top-level [`CallOverlay`] is the single
//! entry point the app shell mounts — it dispatches between the
//! incoming-call modal, the active call view, and the refresh-recovery
//! prompt based on the current [`crate::call::CallState`].

mod call_controls;
mod call_overlay;
mod call_view;
mod incoming_call_modal;
mod network_indicator;
mod recovery_prompt;
mod video_grid;
mod video_tile;

pub use call_controls::CallControls;
pub use call_overlay::CallOverlay;
pub use call_view::CallView;
pub use incoming_call_modal::IncomingCallModal;
pub use network_indicator::NetworkIndicator;
pub use recovery_prompt::CallRecoveryPrompt;
pub use video_grid::VideoGrid;
pub use video_tile::VideoTile;
