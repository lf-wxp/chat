//! Message search state

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResultItem {
  /// Message ID
  pub message_id: String,
  /// Conversation ID
  pub conversation_id: String,
  /// Conversation name
  pub conversation_name: String,
  /// Sender ID
  pub from: String,
  /// Message text preview (highlighted context)
  pub preview: String,
  /// Timestamp
  pub timestamp: i64,
}

/// Message search state
#[derive(Debug, Clone, Default)]
pub struct SearchState {
  /// Search keyword
  pub query: String,
  /// Whether currently searching
  pub is_searching: bool,
  /// Search results list
  pub results: Vec<SearchResultItem>,
  /// Whether to show search panel
  pub show_panel: bool,
  /// In-chat search: matched message ID list
  pub in_chat_matches: Vec<String>,
  /// Currently highlighted match index
  pub in_chat_current_index: usize,
}
