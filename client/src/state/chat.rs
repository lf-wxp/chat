//! Chat and conversation state

use message::chat::ChatMessage;

/// Single conversation
#[derive(Debug, Clone)]
pub struct Conversation {
  /// Conversation ID (peer user ID or room ID)
  pub id: String,
  /// Conversation name
  pub name: String,
  /// Last message preview
  pub last_message: Option<String>,
  /// Last message timestamp
  pub last_time: Option<i64>,
  /// Unread message count
  pub unread_count: u32,
  /// Whether this is a group chat
  pub is_group: bool,
  /// Whether pinned
  pub pinned: bool,
  /// Whether muted
  pub muted: bool,
}

/// Chat state
#[derive(Debug, Clone, Default)]
pub struct ChatState {
  /// Conversation list
  pub conversations: Vec<Conversation>,
  /// Currently active conversation ID
  pub active_conversation_id: Option<String>,
  /// Message list for current conversation
  pub messages: Vec<ChatMessage>,
}
