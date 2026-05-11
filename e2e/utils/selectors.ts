/**
 * Centralised CSS / data-testid selectors used across the E2E suite.
 *
 * Keep this file as the single source of truth so that frontend renames only
 * require a one-line update here instead of grep-and-replace across every spec.
 */

export const sel = {
  // ---- Auth ----
  authPage: '[data-testid="auth-page"]',
  loginForm: '[data-testid="login-form"]',
  loginUsername: '[data-testid="login-username"]',
  loginPassword: '[data-testid="login-password"]',
  loginSubmit: '[data-testid="login-submit"]',
  loginError: '[data-testid="login-error"]',
  registerForm: '[data-testid="register-form"]',
  registerUsername: '[data-testid="register-username"]',
  registerPassword: '[data-testid="register-password"]',
  registerConfirmPassword: '[data-testid="register-confirm-password"]',
  registerSubmit: '[data-testid="register-submit"]',
  registerError: '[data-testid="register-error"]',
  authSwitchToLogin: '[data-testid="auth-switch-to-login"]',
  authSwitchToRegister: '[data-testid="auth-switch-to-register"]',

  // ---- Sidebar / navigation ----
  sidebar: '[data-testid="sidebar"]',
  sidebarSettingsBtn: '[data-testid="sidebar-settings-btn"]',
  sidebarConversationItem: '[data-testid="sidebar-conversation-item"]',
  sidebarConnectionStatus: '[data-testid="sidebar-connection-status"]',
  sidebarRoomSection: '[data-testid="sidebar-room-section"]',
  sidebarRoomItem: '[data-testid="sidebar-room-item"]',
  homeEmpty: '[data-testid="home-empty"]',

  // ---- Online users / discovery ----
  onlineUsersPanel: '[data-testid="online-users-panel"]',
  onlineUsersSearch: '[data-testid="online-users-search"]',
  onlineUserRow: '[data-testid="online-user-row"]',
  userInfoCard: '[data-testid="user-info-card"]',
  userInfoBackdrop: '[data-testid="user-info-backdrop"]',
  userInfoInvite: '[data-testid="user-info-invite"]',
  userInfoConnecting: '[data-testid="user-info-connecting"]',
  userInfoBlock: '[data-testid="user-info-block"]',

  // ---- Invitation ----
  incomingInviteModal: '[data-testid="incoming-invite-modal"]',
  inviteAccept: '[data-testid="invite-accept"]',
  inviteDecline: '[data-testid="invite-decline"]',
  inviteBackdrop: '[data-testid="invite-backdrop"]',

  // ---- Chat view ----
  chatView: '[data-testid="chat-view"]',
  e2eeReadySentinel: '[data-testid="e2ee-ready-sentinel"]',
  chatInputBar: '[data-testid="chat-input-bar"]',
  chatInputTextarea: '[data-testid="chat-input-textarea"]',
  chatInputSend: '[data-testid="chat-input-send"]',
  replyPreviewBar: '[data-testid="reply-preview-bar"]',
  typingIndicator: '[data-testid="typing-indicator"]',
  messageList: '[data-testid="message-list"]',
  messageRow: '[data-testid="message-row"]',
  messageRevoked: '[data-testid="message-revoked"]',
  messageVoice: '[data-testid="message-voice"]',
  messageFile: '[data-testid="message-file"]',
  messageActionReply: '[data-testid="message-action-reply"]',
  messageActionReact: '[data-testid="message-action-react"]',
  messageActionForward: '[data-testid="message-action-forward"]',
  messageActionRevoke: '[data-testid="message-action-revoke"]',
  messageActionCopy: '[data-testid="message-action-copy"]',
  newMessagesBadge: '[data-testid="new-messages-badge"]',
  backToLatestBtn: '[data-testid="back-to-latest"]',

  // ---- Sticker / image / file pickers ----
  stickerPanel: '[data-testid="sticker-panel"]',
  filePickerInput: '[data-testid="file-picker-input"]',
  imagePickerInput: '[data-testid="image-picker-input"]',
  reactionPicker: '[data-testid="reaction-picker"]',
  reactionPickerEmoji: '[data-testid="reaction-picker-emoji"]',

  // ---- Rendered reactions on message bubbles ----
  messageReactions: '[data-testid="message-reactions"]',
  reactionChip: '[data-testid="reaction-chip"]',

  // ---- Forward / image preview ----
  forwardModal: '[data-testid="forward-modal"]',
  imagePreview: '[data-testid="image-preview"]',
} as const;

/** Build a `data-testid` selector dynamically. */
export function testid(id: string): string {
  return `[data-testid="${id}"]`;
}

/** Build a selector for a specific message bubble by its message id. */
export function messageRowById(messageId: string): string {
  return `[data-testid="message-row"][data-message-id="${messageId}"]`;
}
