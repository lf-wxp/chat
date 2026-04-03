//! UI interaction state

/// UI interaction state
#[derive(Debug, Clone, Default)]
pub struct UiState {
  /// Whether sidebar is expanded (mobile/tablet)
  pub sidebar_open: bool,
  /// Currently displayed modal
  pub active_modal: Option<ModalType>,
  /// Toast notification queue
  pub toasts: Vec<Toast>,
  /// Currently generated invite link code
  pub invite_link_code: Option<String>,
  /// Invite link expiration timestamp (milliseconds)
  pub invite_link_expires_at: Option<i64>,
}

/// Modal type
#[derive(Debug, Clone, PartialEq)]
pub enum ModalType {
  /// Create room
  CreateRoom,
  /// Create theater
  CreateTheater,
  /// User profile card
  UserProfile(String),
  /// Connection invitation
  InviteReceived {
    from_user_id: String,
    from_username: String,
    message: Option<String>,
  },
  /// Incoming call
  IncomingCall {
    from_user_id: String,
    from_username: String,
    is_video: bool,
  },
  /// Image preview
  ImagePreview(Vec<u8>),
  /// Confirm dialog
  Confirm {
    title: String,
    message: String,
    on_confirm: String, // Callback identifier
  },
}

/// Toast notification
#[derive(Debug, Clone)]
pub struct Toast {
  pub id: String,
  pub message: String,
  pub toast_type: ToastType,
  pub duration_ms: u32,
}

/// Toast type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
  Success,
  Error,
  Warning,
  Info,
}
