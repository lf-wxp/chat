# Requirement 16: E2E Messaging Test

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a developer, I want comprehensive end-to-end tests for two-client messaging scenarios, so that I can verify the complete user journey — from registration and invitation to conversation management and all message features — works correctly in a real browser environment.

> **Architecture Note:** E2E tests use Playwright to automate two independent browser contexts (simulating two separate users) against a real signaling server instance. Each test scenario exercises the full stack: frontend (Leptos WASM) → WebSocket signaling → WebRTC PeerConnection + DataChannel → message delivery → UI rendering. The signaling server is started as part of the test setup and torn down after all tests complete. Tests run in headless Chromium by default, with an option to run headed for debugging.
>
> **Test Environment Constraint:** Since the server uses in-memory storage (no persistent database), each test suite starts with a clean server state. Tests within a suite share the same server instance but should be designed to be independent (unique usernames per test to avoid conflicts).
>
> **WebRTC in E2E Tests:** Playwright supports WebRTC in Chromium. Tests use `--use-fake-device-for-media-stream` and `--use-fake-ui-for-media-stream` Chromium flags to simulate media devices without requiring real hardware. For DataChannel-based messaging, no special flags are needed — Playwright's Chromium supports full WebRTC DataChannel functionality.

---

## 16.1 Test Infrastructure & Setup

### User Story

As a developer, I want a reliable test infrastructure that starts the signaling server, creates browser contexts, and provides helper utilities, so that I can write E2E tests efficiently without boilerplate.

### Acceptance Criteria

1. WHEN the E2E test suite starts THEN the test infrastructure SHALL:
   - Start the signaling server binary (compiled in release mode) on a random available port
   - Wait for the server to be ready (health check via HTTP GET to the root URL, retry up to 10 times with 500ms intervals)
   - Configure Playwright with two independent browser contexts (Context A and Context B), each with isolated cookies, localStorage, and IndexedDB
   - Set Chromium launch flags: `--use-fake-device-for-media-stream`, `--use-fake-ui-for-media-stream`, `--allow-insecure-localhost`

2. WHEN the E2E test suite completes THEN the test infrastructure SHALL:
   - Close all browser contexts and pages
   - Terminate the signaling server process gracefully (SIGTERM, wait up to 5 seconds, then SIGKILL)
   - Collect and attach server logs to the test report (for debugging failed tests)

3. WHEN a test needs to register and log in a user THEN the test infrastructure SHALL provide a helper function `registerAndLogin(page, username, password)` that:
   - Navigates to the app URL
   - Fills in the registration form (username + password)
   - Submits the form
   - Waits for the main application shell to be visible (sidebar + main content area)
   - Returns the user's display name for assertion purposes

4. WHEN a test needs to establish a connection between two users THEN the test infrastructure SHALL provide a helper function `establishConnection(pageA, pageB, usernameB)` that:
   - On Page A: Finds user B in the online user list, clicks to open the info card, clicks "Send Connection Invitation"
   - On Page B: Waits for the invitation popup, clicks "Accept"
   - Waits for both pages to show the chat interface (DataChannel established)
   - Returns a connection context object for further assertions

5. WHEN a test needs to send a message and verify receipt THEN the test infrastructure SHALL provide a helper function `sendAndVerifyMessage(senderPage, receiverPage, messageContent)` that:
   - On sender page: Types the message in the input box, presses Enter
   - On sender page: Waits for the message to appear in the chat with "sent" status
   - On receiver page: Waits for the message to appear in the chat
   - Returns the message element locators on both pages for further assertions

6. The test infrastructure SHALL set a default test timeout of 30 seconds per test case, with an option to override for longer-running scenarios (e.g., file transfer tests)

7. The test infrastructure SHALL generate unique usernames for each test run using a pattern: `test_user_{testId}_{timestamp}_{random}` to avoid conflicts between parallel test runs

---

## 16.2 User Registration & Login Flow

### User Story

As a developer, I want to verify that user registration and login work correctly end-to-end, so that I can ensure users can enter the application and establish their identity.

### Acceptance Criteria

1. WHEN User A registers with a valid username and password THEN the E2E test SHALL verify:
   - The registration form accepts the input without validation errors
   - After successful registration, the app navigates to the main application shell
   - The sidebar displays User A's username and auto-generated avatar (Identicon)
   - The online user list shows User A as online
   - _Requirement: 10.1, 10.25_

2. WHEN User A registers with a username that is already taken THEN the E2E test SHALL verify:
   - The registration form displays a "Username already exists" error message
   - The user remains on the registration page
   - _Requirement: 10.2_

3. WHEN User A and User B both register and log in THEN the E2E test SHALL verify:
   - User A's online user list shows User B as online
   - User B's online user list shows User A as online
   - Both users' avatars are displayed correctly (Identicon-generated)
   - _Requirement: 9.1, 9.2, 10.25_

4. WHEN User A refreshes the page after logging in THEN the E2E test SHALL verify:
   - The page reloads and automatically restores User A's session (no re-login required)
   - The sidebar displays User A's username and avatar
   - The online user list is repopulated
   - _Requirement: 10.7, 10.8, 10.9_

---

## 16.3 Connection Invitation & Chat Session Establishment

### User Story

As a developer, I want to verify the complete invitation flow from sending to accepting, so that I can ensure two users can establish a P2P chat session.

### Acceptance Criteria

1. WHEN User A clicks User B in the online user list THEN the E2E test SHALL verify:
   - A user info card popup appears showing User B's username, avatar, and online status
   - The info card contains a "Send Connection Invitation" button
   - _Requirement: 9.3_

2. WHEN User A clicks "Send Connection Invitation" THEN the E2E test SHALL verify:
   - User A's UI shows "Invitation sent, waiting for response" status
   - The "Send Connection Invitation" button changes to "Invitation Pending" (disabled state)
   - User B receives an invitation popup/notification card showing User A's username and avatar
   - The invitation popup contains "Accept" and "Decline" buttons
   - _Requirement: 9.4, 9.5, 9.9_

3. WHEN User B clicks "Accept" on the invitation THEN the E2E test SHALL verify:
   - Both User A and User B are navigated to a one-on-one chat interface
   - The chat interface shows the other user's username in the header
   - The message input box is enabled and ready for typing
   - User A's online user list shows User B as "Connected"
   - User B's online user list shows User A as "Connected"
   - An encryption status icon is visible indicating the session is encrypted (E2EE established)
   - _Requirement: 9.6, 9.10, 5.1, 5.6_

4. WHEN User B clicks "Decline" on the invitation THEN the E2E test SHALL verify:
   - User A's UI displays "The other party has declined your invitation" prompt
   - User A's "Send Connection Invitation" button becomes clickable again
   - No chat session is created
   - _Requirement: 9.7_

5. WHEN User A sends an invitation and User B does not respond within 60 seconds THEN the E2E test SHALL verify:
   - User A's UI displays "Invitation has timed out" prompt
   - User A's "Send Connection Invitation" button becomes clickable again
   - _Requirement: 9.8_

6. WHEN User A and User B simultaneously send invitations to each other THEN the E2E test SHALL verify:
   - The system detects the bidirectional invitation and merges them
   - Both users are automatically connected without needing to accept
   - A chat session is established between both users
   - _Requirement: 9.13_

---

## 16.4 Text Message Send & Receive

### User Story

As a developer, I want to verify that text messages are correctly sent, received, and displayed with proper status indicators, so that I can ensure the core chat functionality works end-to-end.

### Acceptance Criteria

1. WHEN User A sends a plain text message "Hello, World!" THEN the E2E test SHALL verify:
   - On User A's page: The message appears right-aligned with User A's background color
   - On User A's page: The message status progresses from "sending" (✓) to "delivered" (✓✓)
   - On User B's page: The message appears left-aligned with the default background color
   - On User B's page: The message shows User A's username and avatar
   - On User B's page: The message timestamp is displayed correctly
   - _Requirement: 2.1a, 2.2, 2.3, 2.3a, 14.2.1_

2. WHEN User A sends a message with Markdown formatting (e.g., `**bold** _italic_ \`code\``) THEN the E2E test SHALL verify:
   - On User B's page: The message renders with proper Markdown formatting (bold text, italic text, inline code)
   - _Requirement: 2.5 (Text Message)_

3. WHEN User A sends a message containing a URL (e.g., `https://example.com`) THEN the E2E test SHALL verify:
   - On User B's page: The URL is rendered as a clickable link
   - The link has appropriate `target="_blank"` and `rel="noopener"` attributes
   - _Requirement: 2.5 (Text Message - URL auto-detection)_

4. WHEN User A sends multiple messages in rapid succession THEN the E2E test SHALL verify:
   - All messages appear on User B's page in the correct order
   - No messages are lost or duplicated
   - Message timestamps are in chronological order
   - _Requirement: 2.2, 11.3 (deduplication)_

5. WHEN User B scrolls a delivered message into the visible viewport THEN the E2E test SHALL verify:
   - On User A's page: The message status updates from "delivered" (✓✓) to "read" (✓✓ blue)
   - _Requirement: 2.3b_

6. WHEN User A is typing a message THEN the E2E test SHALL verify:
   - On User B's page: A "typing..." indicator appears in the chat interface
   - When User A stops typing (or sends the message): The "typing..." indicator disappears
   - _Requirement: 2.8_

---

## 16.5 Message Persistence & Recovery

### User Story

As a developer, I want to verify that messages persist across page refreshes and that the ACK/resend mechanism works correctly, so that I can ensure no messages are lost.

### Acceptance Criteria

1. WHEN User A and User B exchange several messages and User A refreshes the page THEN the E2E test SHALL verify:
   - After refresh, User A's chat history is restored from IndexedDB
   - All previously sent and received messages are displayed correctly
   - Message order and content are preserved
   - _Requirement: 11.1, 11.2, 10.7_

2. WHEN User A refreshes the page during an active chat session THEN the E2E test SHALL verify:
   - User A's page shows a "Restoring connections..." status prompt
   - The WebRTC connection is re-established automatically
   - After recovery, User A can continue sending and receiving messages
   - User B sees "The other party is reconnecting..." status during recovery
   - _Requirement: 10.13, 10.16, 10.40, 10.41_

3. WHEN User B sends a message while User A is refreshing the page THEN the E2E test SHALL verify:
   - After User A's connection recovery completes, the message sent during refresh is delivered via the ACK/resend mechanism
   - The message appears in User A's chat without duplication
   - _Requirement: 10.16a, 11.3_

---

## 16.6 Sticker Message

### User Story

As a developer, I want to verify that sticker messages can be sent, received, and displayed correctly, so that I can ensure the sticker system works end-to-end.

### Acceptance Criteria

1. WHEN User A clicks the sticker panel button THEN the E2E test SHALL verify:
   - The sticker picker panel opens with a grid of available stickers
   - Sticker pack tabs are displayed for category switching
   - A search bar is available at the top of the panel
   - _Requirement: 2.10, 14.2.5_

2. WHEN User A clicks a sticker in the picker THEN the E2E test SHALL verify:
   - The sticker is sent immediately and the picker closes
   - On User A's page: The sticker message appears as a larger image (distinct from regular images)
   - On User B's page: The sticker message appears with the same sticker image
   - The sticker loads correctly (not broken image)
   - _Requirement: 2.5 (Sticker Message), 14.2.5_

3. WHEN User A searches for a sticker by keyword THEN the E2E test SHALL verify:
   - The sticker grid filters to show matching stickers in real-time
   - If no stickers match, a "No stickers found" message is displayed
   - _Requirement: 14.2.5_

---

## 16.7 Voice Message

### User Story

As a developer, I want to verify that voice messages can be recorded, sent, received, and played back correctly, so that I can ensure the voice messaging feature works end-to-end.

### Acceptance Criteria

1. WHEN User A clicks the record button and records a voice message THEN the E2E test SHALL verify:
   - A waveform animation is displayed during recording (Canvas-based)
   - The recording duration timer is visible and incrementing
   - _Requirement: 2.11, 2.5 (Voice Message)_

2. WHEN User A releases the record button to send the voice message THEN the E2E test SHALL verify:
   - On User A's page: A voice message bubble appears with a static waveform visualization and duration label
   - On User B's page: The voice message bubble appears with the same waveform and duration
   - The voice message shows a play/pause button
   - _Requirement: 2.5 (Voice Message), 14.2.2_

3. WHEN User B clicks the play button on a received voice message THEN the E2E test SHALL verify:
   - The play button changes to a pause icon
   - The waveform shows playback progress (colored overlay moving from left to right)
   - The current playback time updates in real-time
   - When playback completes, the button reverts to the play icon
   - _Requirement: 14.2.2_

---

## 16.8 Image Message

### User Story

As a developer, I want to verify that image messages can be sent (via file picker and clipboard paste), received, and previewed correctly, so that I can ensure the image messaging feature works end-to-end.

### Acceptance Criteria

1. WHEN User A sends an image via the file picker THEN the E2E test SHALL verify:
   - On User A's page: The image message appears with a thumbnail preview
   - On User B's page: The image message appears with the same thumbnail
   - The image has proper aspect ratio and border-radius styling
   - _Requirement: 2.5 (Image Message), 14.2.3_

2. WHEN User A pastes an image from the clipboard into the input box THEN the E2E test SHALL verify:
   - A preview confirmation popup appears showing the pasted image
   - After User A confirms, the image is sent as a message
   - On User B's page: The image message appears correctly
   - _Requirement: 2.12_

3. WHEN User B clicks on a received image message THEN the E2E test SHALL verify:
   - A fullscreen image preview modal opens
   - The image is displayed centered with a dark overlay background
   - A close button (X) is visible at the top-right corner
   - Zoom controls (+/-/reset) are available
   - Pressing `Escape` closes the preview modal
   - _Requirement: 14.2.3_

---

## 16.9 Message Context Menu Actions

### User Story

As a developer, I want to verify that message context menu actions (reply, quote, revoke, copy, forward) work correctly, so that I can ensure users can interact with messages as designed.

### Acceptance Criteria

1. WHEN User A right-clicks (or long-presses on mobile) a message THEN the E2E test SHALL verify:
   - A context menu appears with the expected options: Reply, Quote, Copy Text, Forward
   - For User A's own messages (within 2 minutes): The "Revoke" option is also present
   - _Requirement: 2.7, 14.2.1_

2. WHEN User A selects "Reply" from the context menu on User B's message THEN the E2E test SHALL verify:
   - A reply preview bar appears above the input box showing User B's name and a truncated message preview
   - The reply preview bar has a close button (X)
   - When User A types and sends a reply message, the sent message displays a quoted block above the message content
   - On User B's page: The reply message shows the quoted block with User B's original message preview
   - _Requirement: 2.15a, 2.15b, 2.15c, 14.2.1_

3. WHEN User B clicks on the quoted block in a reply message THEN the E2E test SHALL verify:
   - The chat scrolls to the original message
   - The original message is briefly highlighted (flash animation)
   - _Requirement: 2.15d_

4. WHEN User A selects "Quote" from the context menu THEN the E2E test SHALL verify:
   - The quoted text is inserted into the input box in blockquote format (prefixed with `> {sender}: {message_preview}`)
   - User A can add additional text and send the combined message
   - _Requirement: 2.15f_

5. WHEN User A selects "Revoke" on their own message (within 2 minutes) THEN the E2E test SHALL verify:
   - A confirmation dialog appears
   - After confirming, User A's message is replaced with "This message has been revoked" placeholder
   - On User B's page: The same message is replaced with "This message has been revoked" placeholder
   - _Requirement: 2.7a_

6. WHEN User A selects "Revoke" on a message older than 2 minutes THEN the E2E test SHALL verify:
   - The "Revoke" option is NOT present in the context menu (or is disabled)
   - _Requirement: 2.7_

7. WHEN User A selects "Copy Text" from the context menu THEN the E2E test SHALL verify:
   - The message text content is copied to the clipboard
   - A toast notification confirms the copy action
   - _Requirement: 14.2.1_

---

## 16.10 Message Forward

### User Story

As a developer, I want to verify that message forwarding works correctly between conversations, so that I can ensure users can share messages with other contacts.

### Acceptance Criteria

1. WHEN User A selects "Forward" from the context menu THEN the E2E test SHALL verify:
   - A forward target selection modal appears
   - The modal displays a searchable list of active conversations
   - _Requirement: 2.13, 2.13a_

2. WHEN User A selects a target conversation and confirms forwarding THEN the E2E test SHALL verify:
   - The forwarded message appears in the target conversation
   - The forwarded message displays a "Forwarded from {original_sender_name}" header
   - The forwarded message has a visually distinct appearance (forwarded icon, slightly different background)
   - _Requirement: 2.13b, 2.13c, 14.2.1_

3. WHEN a forwarded message is displayed THEN the E2E test SHALL verify:
   - The context menu on the forwarded message does NOT include the "Forward" option (anti-chain-forwarding)
   - _Requirement: 2.13e_

---

## 16.11 Message Reaction (Emoji)

### User Story

As a developer, I want to verify that emoji reactions on messages work correctly, so that I can ensure users can express reactions to messages.

### Acceptance Criteria

1. WHEN User A hovers over a message and clicks the reaction button THEN the E2E test SHALL verify:
   - An emoji picker popup appears
   - The popup displays a grid of commonly used emojis
   - _Requirement: 2.14, 14.2.1_

2. WHEN User A selects an emoji from the picker THEN the E2E test SHALL verify:
   - On User A's page: A reaction pill appears below the message showing the emoji + count "1"
   - On User B's page: The same reaction pill appears below the message
   - The reaction pill on User A's page is highlighted (indicating User A has reacted)
   - _Requirement: 2.14a, 2.14b, 14.2.1_

3. WHEN User B also adds the same emoji reaction THEN the E2E test SHALL verify:
   - The reaction pill count updates to "2" on both pages
   - The reaction pill is highlighted on both User A's and User B's pages
   - _Requirement: 2.14b_

4. WHEN User A clicks the same reaction pill again (toggle off) THEN the E2E test SHALL verify:
   - The reaction pill count decreases to "1" on both pages
   - The reaction pill is no longer highlighted on User A's page
   - _Requirement: 2.14c_

5. WHEN User A adds a different emoji reaction THEN the E2E test SHALL verify:
   - A new reaction pill appears alongside the existing one
   - Both reaction pills are displayed correctly with their respective counts
   - _Requirement: 2.14b_

---

## 16.12 Conversation List & Unread Count

### User Story

As a developer, I want to verify that the conversation list correctly reflects unread message counts and last message previews, so that I can ensure users have an accurate overview of their conversations.

### Acceptance Criteria

1. WHEN User B sends a message to User A while User A is viewing a different conversation (or the room list) THEN the E2E test SHALL verify:
   - User A's sidebar shows the conversation with User B with an unread badge (count: 1)
   - The conversation item shows the last message preview text (truncated)
   - The conversation item shows the timestamp of the last message
   - _Requirement: 2.4, 14.1.2_

2. WHEN User B sends multiple messages while User A is not viewing the conversation THEN the E2E test SHALL verify:
   - The unread badge count increments correctly (e.g., 1 → 2 → 3)
   - The last message preview updates to show the most recent message
   - _Requirement: 2.4_

3. WHEN User A clicks on the conversation with unread messages THEN the E2E test SHALL verify:
   - The unread badge is cleared
   - The chat interface loads with the conversation history
   - A "New Messages" divider is displayed between the last read message and the first unread message
   - _Requirement: 14.1.2, 14.11.5_

---

## 16.13 @Mention

### User Story

As a developer, I want to verify that @mention functionality works correctly in chat, so that I can ensure users can be notified when mentioned.

### Acceptance Criteria

1. WHEN User A types "@" followed by User B's username in the message input THEN the E2E test SHALL verify:
   - An autocomplete dropdown appears showing matching users
   - User A can select User B from the dropdown
   - The @mention is inserted into the message input with proper formatting
   - _Requirement: 2.9_

2. WHEN User A sends a message containing an @mention of User B THEN the E2E test SHALL verify:
   - On User B's page: The @mention is highlighted in the message
   - User B receives a special notification for the @mention
   - _Requirement: 2.9_

---

## 16.14 File Transfer

### User Story

As a developer, I want to verify that file transfer via DataChannel works correctly, so that I can ensure users can share files in chat.

### Acceptance Criteria

1. WHEN User A sends a file (e.g., a small text file or PDF) THEN the E2E test SHALL verify:
   - A file transfer progress bar is displayed on User A's page
   - Transfer speed and estimated remaining time are shown
   - On User B's page: A file message card appears with filename, size, and type icon
   - The file message card has a download button
   - _Requirement: 6.1, 6.2, 6.3, 6.5, 14.2.4_

2. WHEN the file transfer completes THEN the E2E test SHALL verify:
   - The progress bar is replaced with a success indicator
   - User B can click the download button to save the file
   - The downloaded file content matches the original file (integrity check)
   - _Requirement: 6.5, 6.5a, 14.2.4_

3. WHEN User A attempts to send a file exceeding the size limit (100MB) THEN the E2E test SHALL verify:
   - The system displays an error message indicating the file is too large
   - The file is not sent
   - _Requirement: 6.8_

4. WHEN User A sends a file with a potentially dangerous extension (e.g., `.exe`) THEN the E2E test SHALL verify:
   - A security warning dialog appears before sending
   - If User A confirms, the file is sent with a "⚠️ Security Risk" label on the receiver's side
   - _Requirement: 6.8b, 6.8c_

---

## 16.15 Message Revoke with Reply Reference

### User Story

As a developer, I want to verify that revoking a message correctly updates reply references, so that I can ensure the UI handles revoked messages gracefully.

### Acceptance Criteria

1. WHEN User A sends a message, User B replies to it, and then User A revokes the original message THEN the E2E test SHALL verify:
   - User A's original message is replaced with "This message has been revoked"
   - User B's reply message still exists, but the quoted block now shows "Original message has been revoked" in italic gray text
   - On both pages: The quoted block in the reply is updated consistently
   - _Requirement: 2.7a, 2.15e_

---

## 16.16 Chat Session Disconnect & Reconnect

### User Story

As a developer, I want to verify that chat sessions handle disconnection and reconnection gracefully, so that I can ensure users can resume conversations after network interruptions.

### Acceptance Criteria

1. WHEN User B closes their browser tab (simulating disconnect) THEN the E2E test SHALL verify:
   - User A's online user list updates User B's status to "offline"
   - User A's chat interface shows an appropriate disconnection indicator
   - _Requirement: 9.2, 10.11_

2. WHEN User B reopens the app and logs back in THEN the E2E test SHALL verify:
   - User A's online user list shows User B as online again
   - User B can re-establish a connection with User A (via new invitation or automatic recovery)
   - Previous chat history is preserved in User B's IndexedDB
   - _Requirement: 10.7, 10.13, 11.2_

---

## 16.17 Multi-User Chat Scenario

### User Story

As a developer, I want to verify that multi-user chat (3+ participants) works correctly, so that I can ensure group messaging features function in a Mesh topology.

### Acceptance Criteria

> **Note:** This scenario requires a third browser context (Context C) for User C. The test infrastructure SHALL support creating additional browser contexts as needed.

1. WHEN User A invites both User B and User C to a multi-user chat THEN the E2E test SHALL verify:
   - Both User B and User C receive invitation popups
   - When both accept, a multi-user chat session is created with all three participants
   - The chat header shows the participant count (3)
   - _Requirement: 9.12, 2.1b_

2. WHEN User A sends a message in the multi-user chat THEN the E2E test SHALL verify:
   - The message appears on both User B's and User C's pages
   - Message delivery status shows aggregated delivery (e.g., "Delivered to 2/2")
   - _Requirement: 2.2, 2.3d, 11.3_

3. WHEN User C leaves the multi-user chat THEN the E2E test SHALL verify:
   - User A and User B see a notification that User C has left
   - User A and User B can continue chatting without interruption
   - Messages sent after User C leaves are only delivered to remaining participants
   - _Requirement: 2.2a_

---

## 16.18 Theme & Accessibility Verification

### User Story

As a developer, I want to verify that theme switching and basic accessibility features work correctly in the chat interface, so that I can ensure the application meets UI/UX and a11y standards.

### Acceptance Criteria

1. WHEN User A switches from light theme to dark theme THEN the E2E test SHALL verify:
   - The chat interface updates immediately without page refresh
   - Message bubbles, sidebar, and input area use dark theme colors
   - The theme preference persists after page refresh
   - _Requirement: 14.7.1_

2. WHEN User A navigates the chat interface using keyboard only THEN the E2E test SHALL verify:
   - `Tab` moves focus through interactive elements in logical order
   - `Enter` activates focused buttons
   - `Escape` closes open modals/popups
   - All focused elements have visible focus indicators (outline)
   - _Requirement: 14.5.2, Accessibility (a11y) NFR_

3. WHEN a new message arrives THEN the E2E test SHALL verify:
   - The message container has `aria-live="polite"` attribute for screen reader announcement
   - _Requirement: Accessibility (a11y) NFR_

---

## 16.19 Message List Scrolling Behavior

### User Story

As a developer, I want to verify that message list scrolling behaviors (auto-scroll, new message badge, infinite scroll) work correctly, so that I can ensure a smooth reading experience.

### Acceptance Criteria

1. WHEN User A is at the bottom of the message list and User B sends a new message THEN the E2E test SHALL verify:
   - The message list auto-scrolls to reveal the new message
   - _Requirement: 14.11.1_

2. WHEN User A has scrolled up to read older messages and User B sends a new message THEN the E2E test SHALL verify:
   - The message list does NOT auto-scroll (preserves User A's reading position)
   - A "New messages ↓" badge appears at the bottom of the message list
   - Clicking the badge scrolls to the bottom and dismisses the badge
   - _Requirement: 14.11.1_

3. WHEN User A scrolls to the top of the message list (with sufficient chat history) THEN the E2E test SHALL verify:
   - A loading spinner appears at the top
   - Older messages are loaded and prepended above the current messages
   - The scroll position is preserved (the message User A was reading stays in the same viewport position)
   - _Requirement: 14.11.3_

---

## 16.20 E2EE Verification

### User Story

As a developer, I want to verify that end-to-end encryption is correctly established and that messages are encrypted during transport, so that I can ensure user privacy is protected.

### Acceptance Criteria

1. WHEN User A and User B establish a chat session THEN the E2E test SHALL verify:
   - An encryption status icon is displayed in the chat interface indicating E2EE is active
   - _Requirement: 5.1, 5.6_

2. WHEN User A sends a message to User B THEN the E2E test SHALL verify (via debug logs or network inspection):
   - The message content is NOT visible in plain text in WebSocket signaling messages (signaling only carries SDP/ICE, not chat messages)
   - The message is transported via DataChannel (P2P), not via the signaling server
   - _Requirement: 5.2, 5.4, Architecture Constraint_

---

## Dependencies

- **Requires**: Requirement 1 (Signaling), Requirement 2 (Chat), Requirement 5 (E2EE), Requirement 8 (Binary Transport), Requirement 9 (Discovery), Requirement 10 (Auth & Recovery), Requirement 11 (Persistence), Requirement 14 (UI Interaction)
- **Integrates with**: Requirement 6 (File Transfer) for file transfer tests, Requirement 15 (Profile & Permissions) for nickname display verification
- **Test Framework**: Playwright (as specified in Testing Strategy NFR)
- **Build Integration**: Tests are runnable via `makers test-e2e` (as specified in Build & Task Management NFR)

---

## Implementation Notes

- **Playwright Configuration**: Use `playwright.config.ts` at the repository root, configured for Chromium only (Firefox and Safari are not required per Browser Compatibility NFR)
- **Server Lifecycle**: The signaling server binary should be built before E2E tests run (`makers build` as a prerequisite); the test setup starts the server binary as a child process
- **Test Parallelism**: Tests within a single spec file run sequentially (shared server state), but different spec files can run in parallel (each with its own server instance on a different port)
- **Flakiness Mitigation**: Use Playwright's built-in retry mechanism (max 2 retries per test), explicit waits (`waitForSelector`, `waitForResponse`), and avoid timing-based assertions (use polling assertions instead)
- **Screenshot Comparison**: Critical UI tests (theme switching, message rendering) should include Playwright screenshot assertions for visual regression detection
- **CI Integration**: E2E tests should be runnable in CI with `makers test-e2e`, using headless Chromium in a Docker container with Xvfb (or Playwright's built-in Docker image)
