# Requirement 2: Chat System (1-on-1 & Multi-User)

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to chat with one or more users via text, so that I can have one-on-one conversations or group discussions.

> **Architecture Note:** One-on-one chat and multi-user chat share the same message system and UI components. One-on-one chat is a chat session automatically created after two users establish a PeerConnection via connection invitation (Requirement 10); multi-user chat is created via multi-select invitation (Requirement 10.12) or the room system (Requirement 4). Both share identical message send/receive, encryption, persistence, and ACK mechanisms — the only difference is participant count and DataChannel topology (one-on-one uses a single DataChannel, multi-user uses multiple DataChannels under Mesh topology).
>
> **Participant Limit:** Multi-user chat is limited to a maximum of 8 participants (consistent with Mesh topology limit for video calls). This ensures all messages are transmitted via DataChannel P2P with end-to-end encryption, eliminating the complexity of dual transport paths.
>
> **One-on-One Chat Lifecycle:** User A clicks User B in the online user list → sends connection invitation → B accepts → both establish PeerConnection + DataChannel → ECDH key exchange → automatically enter one-on-one chat interface → send/receive messages (via DataChannel P2P transport) → either party closes chat or disconnects → PeerConnection closes → chat history retained in IndexedDB.

## Acceptance Criteria

1a. WHEN a user establishes a PeerConnection with another user via connection invitation (Requirement 10) THEN the system SHALL automatically create a one-on-one chat session, both parties can send and receive messages
1b. WHEN a user creates a multi-user session via multi-select invitation (Requirement 10.12) or the room system (Requirement 4) THEN the system SHALL create a multi-user chat session, all participants can send and receive messages
2. WHEN a user sends a text message in multi-user chat THEN the system SHALL transport the message to all participants via DataChannel P2P (WebSocket is only for signaling, does not carry chat messages); under Mesh topology, the sender SHALL send a message copy to each Peer with an established DataChannel
2a. IF a Peer's DataChannel is unavailable in multi-user chat (offline or disconnected) THEN the system SHALL only deliver messages to currently reachable Peers; when the offline Peer reconnects, the system SHALL detect missing messages via the message ACK mechanism and have the counterpart client automatically resend (see Requirement 11.3)
3. WHEN a user sends a message THEN the system SHALL display message status (sending/sent/delivered/read/send failed), and provide a retry option on send failure
3a. WHEN a message is successfully received by the recipient's DataChannel THEN the system SHALL mark the message status as "delivered" (✓✓) on the sender side via the MessageAck mechanism
3b. WHEN a delivered message scrolls into the recipient's visible viewport THEN the system SHALL send a `MessageRead { message_ids }` message via DataChannel (batched: collect read message IDs over a 500ms window, then send as a single batch); upon receiving the read receipt, the sender SHALL update the message status to "read" (✓✓ in blue) and persist the status to IndexedDB
3c. IF the recipient has disabled read receipts in privacy settings (Requirement 13.3) THEN the system SHALL NOT send `MessageRead` messages; the sender's message status SHALL remain at "delivered" and never transition to "read"
3d. In multi-user chat, read receipts SHALL be sent to the original message sender only (not broadcast to all participants); the sender SHALL display read count (e.g., "Read by 3/5") instead of individual read status
4. WHEN a user receives a new message THEN the system SHALL update the unread message count in the chat list, and display the latest message preview
5. WHEN a user is in a chat THEN the system SHALL support sending the following four core message types:
   - **Text Message**: Supports plain text input, Markdown basic format rendering (bold, italic, code blocks, links), URL auto-detection with clickable links
   - **Sticker Message**: The system SHALL provide a built-in Sticker emoji panel, user clicks a Sticker to send it as an image in the chat, Stickers display at larger size (distinct from regular images); the system SHALL support Sticker pack management with at least one default Sticker pack built-in, Sticker resources stored in WebP/SVG format for size optimization; **Sticker Resource Loading Strategy**: Sticker resource files are **not bundled into the WASM binary**, but deployed as independent static resources under `/assets/stickers/`, **hosted uniformly by the signaling server (Axum)**, all connected clients can access Sticker resources via the same server address; clients load on demand (first time opening the Sticker panel loads the resource manifest and thumbnails via HTTP request, clicking send loads the full image), to avoid increasing WASM initial bundle size; **Sticker Cache & Version Management**: Each Sticker pack's resource manifest JSON file SHALL include a `version` field, the client SHALL use Cache API to cache loaded Sticker resources, and check the resource manifest version number each time the Sticker panel is opened, only re-fetching updated resources when the version changes; **Sticker Loading State & Error Handling**: WHEN user first opens the Sticker panel and resources haven't loaded THEN the system SHALL display a skeleton screen/loading animation; IF Sticker resource loading fails (network error or resource not found) THEN the system SHALL display a "Loading failed" prompt with a retry button
   - **Voice Message**: User long-presses or clicks the record button to start recording, after recording completes it displays as a voice bubble (showing waveform and duration), receiver clicks to play; the system SHALL use Opus encoding to compress voice data for size reduction; the system SHALL limit single voice message maximum duration to 120 seconds (2 minutes), automatically stopping recording and sending when exceeded; **Waveform Visualization Technical Specification**:
     - **Rendering Technology**: The system SHALL use HTML5 Canvas for waveform rendering (instead of SVG or DOM elements) for optimal performance in WASM environment; Canvas avoids frequent DOM operations and provides better rendering performance for real-time animations
     - **Waveform Sampling Precision**: The system SHALL sample audio data into 60-80 waveform bars (balance between visual smoothness and performance); each bar represents a segment of audio amplitude
     - **Waveform Visual Style**: The system SHALL render waveform bars with the following specifications — bar width: 2-3px, bar spacing: 1-2px, bar height range: 4px (minimum) to 32px (maximum), bar color: theme-adaptive (primary color in light theme, lighter shade in dark theme), bar border-radius: 1px (slightly rounded edges)
     - **Real-time Update Frequency**: During recording, the system SHALL update waveform animation at 30fps (every ~33ms) using `requestAnimationFrame`, extracting amplitude data from Web Audio API `AnalyserNode` and redrawing the Canvas
     - **Playback Progress Indicator**: During playback, the system SHALL overlay a semi-transparent progress mask on the waveform bars that have been played, with a moving playhead indicator (vertical line) showing current playback position
     - **Static Waveform Generation**: After recording completes, the system SHALL generate and store a static waveform visualization (as Canvas ImageData or base64 image) from the recorded audio buffer, to avoid re-processing on each render
   - **Image Message**: Supports sending images via file picker or clipboard paste, auto-generates thumbnail preview before sending, displays as thumbnail in chat, click to view original (supports zoom and swipe browsing); the system SHALL support JPEG, PNG, WebP, GIF formats
6. WHEN a user sends a file (non-image/voice/Sticker type) THEN the system SHALL display it as a file card (see Requirement 6)
7. WHEN a user long-presses or right-clicks a message THEN the system SHALL display a context menu supporting reply, quote, revoke (own messages only, within 2 minutes), copy, etc.
7a. WHEN a user revokes a message THEN the system SHALL send a revoke command (`MessageRevoke { message_id }`) to all Peers via DataChannel; the revoke command SHALL be included in the message ACK mechanism (see Requirement 11.3), ensuring all Peers eventually receive the revoke command; upon receiving the revoke command, the receiver SHALL mark the message as "revoked" in IndexedDB and display "This message has been revoked" placeholder text in the UI
8. WHEN the other party is typing THEN the system SHALL display a "typing..." status indicator in the chat interface
9. WHEN a user @mentions a participant THEN the system SHALL highlight the @mentioned message, and send a special notification to the @mentioned user
10. WHEN a user clicks the Sticker panel button THEN the system SHALL pop up a Sticker selection panel, displaying available Stickers in a grid, supporting Sticker pack category switching and search

> **Sticker Pack Directory Structure Convention**: Sticker resources SHALL be organized in the following directory structure:
> - `/assets/stickers/{pack_id}/manifest.json` — Resource manifest
> - `/assets/stickers/{pack_id}/thumb/` — Thumbnail directory
> - `/assets/stickers/{pack_id}/full/` — Full image directory
>
> `manifest.json` format: `{ "pack_id": string, "name": string, "version": string, "stickers": [{ "id": string, "name": string, "tags": string[], "thumb": string, "full": string }] }`

11. WHEN a user records a voice message THEN the system SHALL display real-time waveform animation (via Canvas at 30fps update rate, sampling audio into 60-80 bars, showing amplitude visualization) and recording duration during recording, support cancel recording (swipe up to cancel) and send (release to send); the system SHALL generate and store a static waveform visualization upon recording completion
12. WHEN a user pastes an image into the input box THEN the system SHALL automatically detect image content in the clipboard, display a preview confirmation popup, and send after user confirmation

### Message Forward (Req 2.13)

13. WHEN a user selects "Forward" from the message context menu THEN the system SHALL display a forward target selection modal:
13a. The forward target selection modal SHALL display a searchable list of all active conversations (1-on-1 and multi-user sessions), sorted by recent activity; the user SHALL be able to select one or multiple targets (multi-forward)
13b. WHEN the user confirms forwarding THEN the system SHALL send a `ForwardMessage` via DataChannel to each selected target's Peer(s), containing the original message content, original sender info (`original_sender_id`, `original_sender_name`), and original timestamp (`original_timestamp`)
13c. The forwarded message SHALL be displayed in the target conversation with a "Forwarded from {original_sender_name}" header above the message content, visually distinct from regular messages (e.g., with a forwarded icon and lighter background)
13d. IF the forwarded message is a Sticker, Voice, or Image type THEN the system SHALL forward the message metadata (Sticker pack_id + sticker_id, Voice opus_data + duration, Image thumbnail + metadata) as-is; the receiver SHALL render the forwarded message using the same component as the original message type
13e. The system SHALL support forwarding a single message only (no multi-select batch forward); forwarded messages SHALL NOT be re-forwardable (to prevent infinite forwarding chains — the context menu on a forwarded message SHALL NOT include the "Forward" option)
13f. WHEN forwarding to a multi-user session THEN the system SHALL send the `ForwardMessage` to all Peers in that session via their respective DataChannels (same broadcast mechanism as regular messages under Mesh topology)

### Message Reaction (Req 2.14)

14. WHEN a user clicks the reaction button (emoji face icon) on a message THEN the system SHALL display an emoji picker popup and allow the user to select an emoji to add as a reaction to that message:
14a. The system SHALL send a `MessageReaction` message via DataChannel to all Peers in the session, containing the `target_message_id`, `emoji` (Unicode emoji character), and `action` ("add" or "remove")
14b. WHEN a Peer receives a `MessageReaction` message THEN the system SHALL update the reaction display on the target message: show reaction pills below the message content, each pill displaying the emoji + count of users who reacted with that emoji
14c. IF the current user has already added a specific emoji reaction THEN clicking the same emoji again (either via the emoji picker or by clicking the reaction pill) SHALL remove the reaction (toggle behavior), sending a `MessageReaction` with `action: "remove"`
14d. The system SHALL limit reactions to a maximum of 20 unique emoji types per message; IF the limit is reached THEN the emoji picker SHALL display a "Maximum reactions reached" tooltip and prevent adding new emoji types (existing emoji reactions can still be toggled)
14e. The system SHALL persist reactions in IndexedDB alongside the message record; each message's reaction data SHALL be stored as a map of `{ emoji: Vec<UserId> }` (emoji → list of users who reacted)
14f. Reaction messages SHALL use "best-effort" delivery (similar to typing indicators), no ACK/resend mechanism needed; WHEN a Peer reconnects THEN the system SHALL NOT resend historical reactions (reactions are synced via IndexedDB state on the local client)
14g. In multi-user sessions, `MessageReaction` messages SHALL be broadcast to all Peers (same as regular chat messages under Mesh topology)

### Message Reply & Quote Display (Req 2.15)

15. WHEN a user selects "Reply" from the message context menu THEN the system SHALL:
15a. Display a reply preview bar above the message input box, showing the original message sender name and a truncated preview of the original message content (max 50 characters); the reply preview bar SHALL include a close button (X) to cancel the reply
15b. WHEN the user sends a message while the reply preview bar is active THEN the system SHALL include a `reply_to` field in the `ChatText` message payload, containing the `original_message_id` and `original_sender_name`
15c. WHEN a message with a `reply_to` field is displayed in the chat THEN the system SHALL render a quoted message block above the message content, showing: original sender name (bold), original message preview (truncated to 2 lines, max 100 characters), and a left border accent line (4px, primary color)
15d. WHEN the user clicks on the quoted message block THEN the system SHALL scroll the chat history to the original message and briefly highlight it (flash animation, 1 second); IF the original message is not in the currently loaded message range THEN the system SHALL load older messages from IndexedDB until the original message is found
15e. IF the original message has been revoked THEN the quoted message block SHALL display "Original message has been revoked" in italic gray text
15f. WHEN a user selects "Quote" from the context menu THEN the system SHALL insert the quoted message text into the input box in a blockquote format (prefixed with "> {sender}: {message_preview}\n"), allowing the user to add their own text before sending
