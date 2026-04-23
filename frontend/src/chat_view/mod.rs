//! Chat view module.
//!
//! UI layer for Task 16. Each component lives in its own file following
//! the "one component per file" project rule:
//!
//! * [`chat_view::ChatView`] — root container that composes the full
//!   chat pane (message list + typing indicator + input bar + all
//!   overlays).
//! * [`message_list::MessageList`] — scrollable reactive message list
//!   with auto-scroll / new-messages badge / back-to-latest.
//! * [`message_bubble::MessageBubble`] — single-row bubble with hover
//!   actions and content-type dispatch.
//! * [`input_bar::InputBar`] — composer with typing / mention / reply /
//!   attachment triggers.
//! * [`sticker_panel::StickerPanel`], [`voice_recorder::VoiceRecorder`],
//!   [`image_picker::ImagePicker`] — overlay surfaces triggered by the
//!   input bar.
//! * [`forward_modal::ForwardModal`] — modal for the forward command.
//! * [`reaction_picker::ReactionPicker`] — emoji overlay for the
//!   reaction action.
//! * [`image_preview::ImagePreviewOverlay`] — full-screen image viewer.
//! * [`typing_indicator::TypingIndicator`] — inline typing strip.
//! * [`helpers`] — pure formatting / mention rendering helpers.

pub mod forward_modal;
pub mod helpers;
pub mod image_picker;
pub mod image_preview;
pub mod input_bar;
pub mod message_bubble;
pub mod message_list;
pub mod reaction_picker;
pub mod sticker_cache;
pub mod sticker_panel;
pub mod typing_indicator;
pub mod view_root;
pub mod voice_recorder;

pub use view_root::ChatView;
