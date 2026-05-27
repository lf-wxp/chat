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
  sidebarSearchInput: '[data-testid="sidebar-search-input"]',
  sidebarConversationItem: '[data-testid="sidebar-conversation-item"]',
  sidebarConnectionStatus: '[data-testid="sidebar-connection-status"]',
  sidebarRoomSection: '[data-testid="sidebar-room-section"]',
  sidebarRoomItem: '[data-testid="sidebar-room-item"]',
  sidebarRoomJoinBtn: '[data-testid="sidebar-room-join-btn"]',
  sidebarRoomCreateBtn: '[data-testid="sidebar-room-create-btn"]',
  sidebarSectionPinned: '[data-testid="sidebar-section-pinned"]',
  sidebarSectionActive: '[data-testid="sidebar-section-active"]',
  sidebarSectionArchived: '[data-testid="sidebar-section-archived"]',
  sidebarConversationActions: '[data-testid="sidebar-conversation-actions-btn"]',
  sidebarConversationMenu: '[data-testid="sidebar-conversation-menu"]',
  sidebarConversationMenuPin: '[data-testid="sidebar-conversation-menu-pin"]',
  sidebarConversationMenuMute: '[data-testid="sidebar-conversation-menu-mute"]',
  sidebarConversationMenuArchive: '[data-testid="sidebar-conversation-menu-archive"]',
  sidebarConversationMenuDelete: '[data-testid="sidebar-conversation-menu-delete"]',
  sidebarDeleteModal: '[data-testid="sidebar-delete-modal"]',
  sidebarDeleteModalConfirm: '[data-testid="sidebar-delete-modal-confirm"]',
  sidebarDeleteModalCancel: '[data-testid="sidebar-delete-modal-cancel"]',
  homeEmpty: '[data-testid="home-empty"]',

  // ---- Room create modal ----
  createRoomModal: '[data-testid="create-room-modal"]',
  createRoomName: '[data-testid="create-room-name"]',
  createRoomDescription: '[data-testid="create-room-description"]',
  createRoomTypeChat: '[data-testid="room-type-chat"]',
  createRoomTypeTheater: '[data-testid="room-type-theater"]',
  createRoomSubmit: '[data-testid="create-room-submit"]',
  createRoomCancel: '[data-testid="create-room-cancel"]',

  // ---- Online users / discovery ----
  onlineUsersPanel: '[data-testid="online-users-panel"]',
  onlineUsersSearch: '[data-testid="online-users-search"]',
  onlineUserRow: '[data-testid="online-user-row"]',
  userInfoCard: '[data-testid="user-info-card"]',
  // The user-info card is rendered via the shared ModalWrapper, which
  // tags its backdrop with `modal-wrapper-backdrop`. The card-specific
  // selector targets that wrapper but scoped to the user-info dialog
  // so other modals don't match.
  userInfoBackdrop:
    '[data-testid="modal-wrapper-backdrop"]:has([data-testid="user-info-card"])',
  userInfoInvite: '[data-testid="user-info-invite"]',
  userInfoConnecting: '[data-testid="user-info-connecting"]',
  userInfoBlock: '[data-testid="user-info-block"]',

  // ---- Invitation ----
  incomingInviteModal: '[data-testid="incoming-invite-modal"]',
  inviteAccept: '[data-testid="invite-accept"]',
  inviteDecline: '[data-testid="invite-decline"]',
  // Incoming-invite modal is now hosted via the shared `ModalWrapper`,
  // which tags its backdrop with `modal-wrapper-backdrop`. Scope the
  // selector to the inner dialog so other modals don't match.
  inviteBackdrop:
    '[data-testid="modal-wrapper-backdrop"]:has([data-testid="incoming-invite-modal"])',

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
  replyBlock: '[data-testid="reply-block"]',

  // ---- Sticker / image / file pickers ----
  stickerPanel: '[data-testid="sticker-panel"]',
  stickerPanelItem: '[data-testid="sticker-panel-item"]',
  messageSticker: '[data-testid="message-sticker"]',
  messageImage: '[data-testid="message-image"]',
  filePickerInput: '[data-testid="file-picker-input"]',
  imagePickerInput: '[data-testid="image-picker-input"]',
  reactionPicker: '[data-testid="reaction-picker"]',
  reactionPickerEmoji: '[data-testid="reaction-picker-emoji"]',

  // ---- Voice recorder ----
  voiceRecorder: '[data-testid="voice-recorder"]',
  voiceRecorderRecord: '[data-testid="voice-recorder-record"]',
  voiceRecorderSend: '[data-testid="voice-recorder-send"]',
  voiceRecorderCancel: '[data-testid="voice-recorder-cancel"]',

  // ---- Theater ----
  theaterPage: '[data-testid="theater-page"]',
  theaterSourcePicker: '[data-testid="theater-source-picker"]',
  theaterSourceLocal: '[data-testid="theater-source-local"]',
  theaterSourceScreen: '[data-testid="theater-source-screen"]',
  theaterSourceUrl: '[data-testid="theater-source-url"]',
  theaterVideoPlayer: '[data-testid="theater-video-player"]',
  theaterVideo: '[data-testid="theater-video"]',
  theaterSourceLocalInput: '[data-testid="theater-source-local-input"]',
  theaterChatPanel: '[data-testid="theater-chat-panel"]',
  theaterChatInput: '[data-testid="theater-chat-input"]',
  theaterChatSend: '[data-testid="theater-chat-send"]',
  theaterTabChat: '[data-testid="theater-tab-chat"]',
  theaterTabMembers: '[data-testid="theater-tab-members"]',
  theaterPlaybackControls: '[data-testid="theater-playback-controls"]',
  theaterPlayPause: '[data-testid="theater-play-pause"]',
  theaterSeekBar: '[data-testid="theater-seek-bar"]',
  theaterMuteToggle: '[data-testid="theater-mute-toggle"]',
  theaterVolumeSlider: '[data-testid="theater-volume-slider"]',
  theaterFullscreenToggle: '[data-testid="theater-fullscreen-toggle"]',
  theaterMemberPanel: '[data-testid="theater-member-panel"]',
  theaterMuteAll: '[data-testid="theater-mute-all"]',
  theaterLoadBanner: '[data-testid="theater-load-banner"]',
  theaterGraceBanner: '[data-testid="theater-grace-banner"]',
  theaterGraceLeave: '[data-testid="theater-grace-leave"]',
  theaterPanelToggle: '[data-testid="theater-panel-toggle"]',
  theaterCopyrightNotice: '[data-testid="theater-copyright-notice"]',
  theaterSubtitleOverlay: '[data-testid="theater-subtitle-overlay"]',
  danmakuCanvas: '[data-testid="danmaku-canvas"]',
  danmakuInput: '[data-testid="danmaku-input"]',
  danmakuInputField: '[data-testid="danmaku-input-field"]',
  danmakuInputSend: '[data-testid="danmaku-input-send"]',
  danmakuInputPosition: '[data-testid="danmaku-input-position"]',
  danmakuSettingsPanel: '[data-testid="danmaku-settings-panel"]',
  danmakuVisibleToggle: '[data-testid="danmaku-visible-toggle"]',
  danmakuOpacitySlider: '[data-testid="danmaku-opacity-slider"]',
  danmakuFontSize: '[data-testid="danmaku-font-size"]',
  danmakuSpeed: '[data-testid="danmaku-speed"]',

  // ---- File transfer card internals ----
  fileProgress: '[data-testid="file-progress"]',
  fileCancel: '[data-testid="file-cancel"]',
  filePause: '[data-testid="file-pause"]',
  fileResume: '[data-testid="file-resume"]',
  fileDownload: '[data-testid="file-download"]',
  fileDownloadDangerBtn: '[data-testid="file-download-danger-btn"]',
  fileReReceive: '[data-testid="file-re-receive"]',
  fileHashMismatch: '[data-testid="file-hash-mismatch"]',
  fileDangerBadge: '[data-testid="file-danger-badge"]',
  fileExtDanger: '[data-testid="file-ext-danger"]',

  // ---- Dialog (custom confirm / alert modal) ----
  dialog: '[data-testid="dialog"]',
  dialogMessage: '[data-testid="dialog-message"]',
  dialogOk: '[data-testid="dialog-ok"]',
  dialogCancel: '[data-testid="dialog-cancel"]',

  // ---- Rendered reactions on message bubbles ----
  messageReactions: '[data-testid="message-reactions"]',
  reactionChip: '[data-testid="reaction-chip"]',

  // ---- Forward / image preview ----
  forwardModal: '[data-testid="forward-modal"]',
  imagePreview: '[data-testid="image-preview"]',

  // ---- Call (audio / video) ----
  callStartBtn: '[data-testid="call-start-btn"]',
  incomingCallModal: '[data-testid="incoming-call-modal"]',
  callAcceptBtn: '[data-testid="call-accept-btn"]',
  callDeclineBtn: '[data-testid="call-decline-btn"]',
  callView: '[data-testid="call-view"]',
  callEndBtn: '[data-testid="call-end-btn"]',
  videoTile: '[data-testid="video-tile"]',
  videoTileLocal: '[data-testid="video-tile-local"]',
  videoTileRemote: '[data-testid="video-tile-remote"]',

  // ---- Room member list ----
  roomMemberList: '[data-testid="room-member-list"]',
  roomMemberRow: '[data-testid="room-member-row"]',
  roomMemberMenu: '[data-testid="room-member-menu"]',
  roomMemberMenuItem: '[data-testid="room-member-menu-item"]',
  roomMemberMenuItemMention: '[data-testid="room-member-menu-item"][data-action="mention"]',

  // ---- @Mention rendering ----
  mentionHighlight: '[data-testid="mention-highlight"]',
  messageRowMentionsMe: '[data-testid="message-row"][data-mentions-me="true"]',

  // ---- Nickname editor (Settings drawer Account section) ----
  nicknameEditor: '[data-testid="nickname-editor"]',
  nicknameEditorInput: '[data-testid="nickname-editor-input"]',
  nicknameEditorSave: '[data-testid="nickname-editor-save"]',
  nicknameEditorError: '[data-testid="nickname-editor-error"]',

  settingsPageSelector: '[data-testid="settings-page"]',

} as const;

/** Build a `data-testid` selector dynamically. */
export function testid(id: string): string {
  return `[data-testid="${id}"]`;
}

/** Build a selector for a specific message bubble by its message id. */
export function messageRowById(messageId: string): string {
  return `[data-testid="message-row"][data-message-id="${messageId}"]`;
}

/** Build a selector for a room member row by display nickname. */
export function roomMemberRowByNickname(nickname: string): string {
  return `[data-testid="room-member-row"][data-nickname="${nickname}"]`;
}
