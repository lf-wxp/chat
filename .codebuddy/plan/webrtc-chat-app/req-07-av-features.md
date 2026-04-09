# Requirement 7: Audio/Video Mode Switching & Common Features

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to flexibly switch audio/video modes during a call, and use common chat application features.

## Acceptance Criteria

1. WHEN a user clicks the "Voice Only" button during a video call THEN the system SHALL smoothly close the video track, switching to voice call mode without re-establishing the connection
2. WHEN a user clicks the "Enable Video" button during a voice call THEN the system SHALL request camera permission and add a video track without re-establishing the connection
3. WHEN a user is in a call THEN the system SHALL provide a floating Picture-in-Picture (PiP) mode, allowing the user to continue the call while browsing other pages
4. WHEN a user receives an incoming call THEN the system SHALL display an incoming call notification popup, including caller info and accept/decline buttons
5. WHEN a call ends THEN the system SHALL display call duration statistics
6. WHEN a user is in a chat THEN the system SHALL support message search functionality:
   - **Search Scope**: Default searches current session's history messages, supports switching to "All Sessions" for global search
   - **Search Method**: Supports keyword fuzzy matching (contains match), case-insensitive
   - **Search Implementation**: Based on IndexedDB-stored message records for client-side local search, implemented via IndexedDB record traversal + in-memory filtering (for large message volumes, consider introducing a lightweight client search library like MiniSearch to build an inverted index for performance optimization); since the frontend is Rust WASM, search logic should prioritize pure Rust implementation (e.g., `Vec<Message>` in-memory filtering + simple inverted index), avoiding the extra complexity of calling JS search libraries via `wasm-bindgen`
   - **Search Performance Targets**: The system SHALL ensure search response time < 500ms for up to 10,000 messages, < 2s for up to 50,000 messages (if performance targets are exceeded, SHALL introduce inverted index optimization); **Test Benchmark**: Measured on a mid-range device (4-core CPU, 8GB RAM) running Chrome latest stable version; WASM runtime memory limit is browser default (typically 2-4GB), search implementation SHALL be mindful of WASM linear memory growth strategy, avoiding loading too many messages into memory at once
   - **Search Pagination & Memory Strategy**: The system SHALL implement a paginated loading strategy for search — load messages from IndexedDB in batches of 5,000 records per page, perform in-memory filtering on each batch, and aggregate results incrementally; WHEN search results reach 50 items THEN the system SHALL stop loading further batches and display results (user can click "Load more results" to continue searching remaining batches); the system SHALL release each batch's memory after filtering (drop the `Vec<Message>` after extracting matches) to prevent WASM linear memory from growing unboundedly; IF total message count exceeds 50,000 THEN the system SHALL build and maintain a lightweight inverted index (stored in IndexedDB as a separate object store, updated incrementally on each new message) to avoid full-scan searches
   - **Result Display**: Search results displayed as a list showing message summary, sender, time, with keyword highlighting; clicking a search result SHALL jump to the corresponding message's position in the chat history
   - **Result Sorting**: Search results SHALL be sorted by relevance score (number of keyword matches) descending first, then by message timestamp descending (newest first) for results with equal relevance; global search SHALL group results by conversation, sorted by the conversation's most recent match timestamp descending
7. WHEN a user is in the chat list THEN the system SHALL support pinning conversations and do-not-disturb settings
7a. **Pinning**: WHEN a user right-clicks (desktop) or long-presses (mobile) a conversation in the chat list THEN the system SHALL display a context menu with a "Pin" / "Unpin" option; pinned conversations SHALL always appear at the top of the chat list, above unpinned conversations, with a subtle pin icon indicator
7b. **Pin Sorting**: Multiple pinned conversations SHALL be sorted by the time they were pinned (most recently pinned first); within the pinned section, the order SHALL NOT change based on new message activity (to maintain user's intentional ordering)
7c. **Pin Limit**: The system SHALL allow a maximum of 5 pinned conversations; IF the user attempts to pin a 6th conversation THEN the system SHALL display a toast: "Maximum 5 pinned conversations. Please unpin one first."
7d. **Pin Persistence**: Pinned conversation state SHALL be persisted to IndexedDB (stored as a list of `{ session_id, pinned_at_timestamp }`), surviving page refresh and browser restart
7e. **Do-Not-Disturb**: WHEN a user enables do-not-disturb for a conversation THEN the system SHALL suppress notification sounds and browser Notification API popups for that conversation; the conversation SHALL still display unread message count badge, but the badge SHALL use a muted color (gray instead of red)
7f. **Archive** (optional): WHEN a user selects "Archive" from the conversation context menu THEN the system SHALL move the conversation to an "Archived" section at the bottom of the chat list (collapsed by default); archived conversations SHALL NOT appear in the main list unless they receive a new message (which auto-unarchives them)
8. WHEN a user uses the app THEN the system SHALL push new message notifications via the browser Notification API (requires user authorization)
