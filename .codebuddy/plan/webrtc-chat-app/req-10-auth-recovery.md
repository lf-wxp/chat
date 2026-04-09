# Requirement 10: User Auth, Session Management & Refresh Recovery

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want a basic identity authentication mechanism, so that I have a unique identity in chat without needing a complex registration process. **More importantly, when I refresh the page (not actively logging out, not token expired), my login state and chat connections should automatically recover, rather than returning to the initial state.**

> **Constraint:** User registration and login information is **NOT persisted on the server** (no database). The server only maintains current active user session information in memory (`DashMap<UserId, UserSession>`), all user data is cleared on server restart, users need to re-register/login. This is a lightweight temporary session model suitable for P2P chat scenarios.

> **Core Challenge:** Page refresh causes all JavaScript runtime state to be lost, including WebRTC `PeerConnection`, `DataChannel`, encryption keys, etc. Since WebRTC connections cannot be serialized and persisted, all connections must be re-negotiated via the signaling server after refresh. A complete "connection recovery protocol" is needed to achieve seamless recovery transparent to the user.

## Acceptance Criteria

### 10.1 Basic Authentication

1. WHEN a user first visits the app THEN the system SHALL display a register/login interface, requiring username and password input
2. WHEN a user registers THEN the server SHALL store user info (username + password hash) in memory (`DashMap`), not writing to any persistent storage (database/file); IF the username is already taken (already exists in memory) THEN the system SHALL prompt "Username already exists"
3. WHEN a user logs in successfully THEN the system SHALL generate a JWT Token (containing AES-encrypted user data) and store it on the client (localStorage), subsequent WebSocket connections carry the Token for authentication; the server records the user's active session in memory
4. IF the Token is expired or invalid (e.g., server restart causes in-memory user data loss and JWT key change) THEN the system SHALL require the user to re-register/login
5. WHEN a user is online THEN the system SHALL display online status in the user list (online/offline/busy/away)
6. WHEN a user sets personal status (busy/away/custom signature) THEN the system SHALL broadcast the status synchronously to all online users
6a. WHEN a user has no activity for 5 minutes (no mouse movement, no keyboard input, no touch events) THEN the system SHALL automatically switch the user's status to "away" and broadcast to all online users; WHEN the user resumes activity THEN the system SHALL automatically switch status back to "online" (unless the user previously manually set "busy" status, in which case no automatic switch)

### 10.2 Identity Recovery After Page Refresh

7. WHEN a user refreshes the page (not actively logging out, Token not expired) THEN the client SHALL read Token and user info from localStorage, automatically restore user state (user_id, username, token), and immediately initiate a WebSocket connection carrying the Token for TokenAuth authentication
8. WHEN the server receives a TokenAuth request THEN the server SHALL decrypt and restore the user session from the JWT (stateless authentication), return AuthSuccess, and re-register the user in the online connection table
9. WHEN TokenAuth succeeds THEN the server SHALL push the latest online user list (UserListUpdate) and room list (RoomListUpdate) to the user, synchronizing client state with the server

### 10.3 WebRTC Connection Recovery After Page Refresh

> **Design Approach:** Since WebRTC PeerConnection cannot survive page refresh, adopt a "server tracking + client proactive rebuild" strategy:
> 1. **Server maintains active Peer list**: The server maintains an `active_peers: HashSet<UserId>` list for each user, recording the peer users with whom the user currently has established WebRTC connections
> 2. **Server delivers Peer list after refresh**: After TokenAuth succeeds, the server delivers the user's active_peers list to the client
> 3. **Client rebuilds connections one by one**: After receiving the Peer list, the client sequentially performs SDP negotiation with each Peer, establishing new PeerConnections and DataChannels
> 4. **E2EE key re-negotiation**: Each new DataChannel re-executes ECDH key exchange

10. WHEN a user successfully establishes a WebRTC PeerConnection with another user THEN both clients SHALL notify the server via the signaling server to record the Peer relationship (`PeerEstablished { from, to }`), the server SHALL add each other to both parties' active_peers lists
11. WHEN a WebRTC PeerConnection closes (normal close or ICE failure) THEN the client SHALL notify the server via the signaling server to remove the Peer relationship (`PeerClosed { from, to }`), the server SHALL remove each other from both parties' active_peers lists
12. WHEN a user's TokenAuth succeeds THEN the server SHALL send an `ActivePeersList { peers: Vec<PeerId> }` signaling message, informing the client of the Peer list that had established connections before the refresh
13. WHEN the client receives ActivePeersList THEN the client SHALL initiate SDP Offers to Peers in the list with limited concurrency (max 2-3 simultaneous SDP negotiations), re-establishing PeerConnections and DataChannels; each connection established SHALL re-execute ECDH key exchange to restore E2EE (using limited concurrency rather than strict serial to accelerate recovery in multi-Peer scenarios; concurrency limit of 2-3 to avoid resource contention in browser WebRTC implementations)
14. WHEN a peer user receives an SDP Offer from a refreshed user THEN the peer SHALL first close the old PeerConnection with that user (if it exists), then accept the new SDP Offer to establish a new connection
15. IF a Peer has gone offline during the refresh THEN the client SHALL skip reconnection for that Peer and remove it from the local active_peers list; the server SHALL automatically clean up the corresponding record in active_peers when detecting a Peer going offline
16. WHEN all Peer reconnections complete (or timeout and skip) THEN the client SHALL restore to the pre-refresh chat interface state, the user can continue sending and receiving messages normally
16a. WHEN connection recovery completes THEN the system SHALL immediately trigger the message ACK synchronization mechanism (see Requirement 11.3), automatically detecting and resending messages lost during the refresh — the sender SHALL check the unacknowledged message queue for each Peer, automatically resending all unacknowledged messages to recovered Peers; the receiver SHALL deduplicate based on `message_id`, ensuring no duplicate display

> **Connection Recovery Message Loss Window:** Page refresh causes JavaScript runtime state loss, creating a message loss window from the moment refresh starts to connection recovery completion. During this window, any messages sent by peers will be lost (no running code to receive them). The ACK mechanism handles recovery: senders track unacknowledged messages, receivers track received message_ids. After recovery, senders automatically resend unacknowledged messages, receivers deduplicate. Typical recovery time is 3-10 seconds (depending on Peer count and network conditions), during which peers see "The other party is reconnecting..." status.

### 10.4 Room State Recovery After Page Refresh

17. WHEN a user was in a room before refresh THEN the client SHALL persist the current room ID to localStorage (`active_room_id`)
18. WHEN TokenAuth succeeds after page refresh THEN the client SHALL read active_room_id from localStorage, if it exists then automatically send a JoinRoom signaling to rejoin the room
19. WHEN the server receives a JoinRoom request from a refreshed user THEN the server SHALL detect whether the user is already a room member (not removed during refresh), if so directly restore member identity rather than joining as a new member; if already removed then process as normal join flow
20. WHEN room rejoin succeeds THEN the server SHALL push the latest room member list to the user, the client SHALL re-establish WebRTC PeerConnections with other room members (reusing the 10.3 connection recovery flow)

### 10.5 Call State Recovery After Page Refresh

21. WHEN a user was in an audio/video call before refresh THEN the client SHALL persist call state (call room ID, media type audio/video) to localStorage (`active_call`)
22. WHEN WebRTC connection recovery completes after page refresh THEN the client SHALL read active_call info from localStorage, if it exists then first display a "Resume previous call?" confirmation popup (including call type and counterpart info); upon user confirmation SHALL re-request media permissions (camera/microphone) and add media tracks to the recovered PeerConnection; if user cancels then recover connection in text-only chat mode
23. IF the user denies media permission request (browser may re-prompt for permission after refresh) THEN the system SHALL recover connection in text-only chat mode, and notify the counterpart of the user's media state change
24. WHEN call recovery is in progress THEN the peer user SHALL see a "The other party is reconnecting..." status prompt, rather than immediately determining the call has ended

### 10.6 User Avatar & Profile

25. WHEN a user registers successfully THEN the system SHALL auto-generate a default avatar based on the username (using Identicon / initial letter avatar algorithm, generating SVG/Canvas image on the client), no manual upload needed
26. WHEN a user modifies their avatar in personal settings THEN the system SHALL support uploading from local (JPEG/PNG/WebP, max 128KB), cropping to square after upload and generating Base64 encoding stored in localStorage; **Avatar exchange uses lazy loading strategy**: WHEN WebRTC DataChannel is established THEN the system SHALL first exchange basic user info (username, avatar SHA-256 hash), the peer SHALL check local cache (IndexedDB) for avatar data matching that hash; IF local cache hits THEN use cached avatar directly without requesting transfer; IF local cache misses THEN the peer SHALL request full avatar data via DataChannel, the sender SHALL transfer the avatar asynchronously (not blocking the first message send/receive)
27. WHEN a user modifies their status signature in personal settings THEN the system SHALL broadcast the signature synchronously to all online users (via signaling server)
28. WHEN a user closes their camera during a video call THEN the system SHALL display their avatar as a placeholder in that user's video area

### 10.7 Multi-Device Login Strategy

29. The system SHALL NOT support simultaneous multi-device login for the same user account; WHEN a user logs in on a new device while already logged in on another device THEN the server SHALL send a `SessionInvalidated` message to the old device, forcibly disconnecting the old session
30. WHEN the old device receives SessionInvalidated THEN the system SHALL display "Your account has logged in on another device" prompt, clear all local state (localStorage, close connections), and redirect to login page
31. This single-device-per-account policy simplifies session management and avoids synchronization complexity in a P2P architecture without a persistent backend database

> **Rationale:** Supporting multi-device login requires complex message synchronization, state consistency, and device management. In a P2P architecture with no persistent backend database, multi-device support would significantly increase implementation complexity. The single-device policy is a deliberate trade-off for architectural simplicity.

### 10.8 Local Data Cleanup

32. WHEN a user clicks "Clear All Local Data" in personal settings THEN the system SHALL display a confirmation dialog warning that all local chat history, cached avatars, and preferences will be permanently deleted; upon user confirmation, the system SHALL clear all data in IndexedDB (chat records, avatar cache, search index) and all keys in localStorage (auth_token, active_room_id, active_call, theme preference, language preference, etc.), then redirect to the login page
33. WHEN a user clicks "Clear Chat History" in personal settings THEN the system SHALL allow selective clearing: clear all sessions, clear a specific session, or clear messages older than a specified date; the system SHALL display the storage space to be freed before clearing, and execute the clearing after user confirmation

### 10.9 Client State Persistence Strategy

34. The client SHALL persist the following state to localStorage for recovery after refresh:
    - `auth_token`: JWT Token
    - `auth_user_id`: User ID
    - `auth_username`: Username
    - `auth_avatar`: User avatar (Base64 encoded, default is Identicon-generated SVG)
    - `active_room_id`: Current room ID (cleared when leaving room)
    - `active_call`: Current call state JSON (cleared when call ends)
    - `active_conversation_id`: Current active chat session ID
35. WHEN a user actively logs out THEN the client SHALL execute the following complete logout flow:
    - a. Close all active WebRTC PeerConnections (triggering `PeerClosed` signaling to notify peers)
    - b. Close all DataChannels
    - c. Stop all media tracks (camera/microphone/screen sharing)
    - d. Send `UserLogout` message via signaling server, the server SHALL clean up the user's active_peers list, room member identity, and broadcast the user offline event to all online users
    - e. Close WebSocket connection
    - f. Clear all persisted state in localStorage (auth_token, active_room_id, active_call, etc.)
    - g. Redirect to login page
36. WHEN the user's active logout flow completes THEN the system SHALL ensure all peer users have received the user's offline notification, peer users' online user lists and chat interfaces SHALL update the user's status to "offline" accordingly

### 10.10 Server Restart Scenario

37. WHEN the server restarts THEN all in-memory user data, room data, session data, active_peers data SHALL be cleared, all online users' WebSocket connections will disconnect
38. WHEN the client detects WebSocket disconnection and reconnection fails (server restart scenario) THEN the client SHALL attempt to re-authenticate using the Token in localStorage; IF the server JWT key has not changed THEN authentication succeeds, user does not need to re-login (but active_peers and room data are lost, user needs to re-establish connections); IF the JWT key has changed THEN guide the user to re-register/login
39. WHEN the client's TokenAuth succeeds after server restart but active_peers and room data are lost THEN the client SHALL display a global prompt "Server has restarted, previous chat connections and rooms are invalid, please re-establish connections", and automatically clear `active_room_id` and `active_call` state in localStorage; after the online user list refreshes, users can restore chat by re-sending connection invitations

### 10.11 Connection Recovery User Experience

40. WHEN connection recovery is in progress after page refresh THEN the client SHALL display a "Restoring connections..." global status prompt in the UI (non-blocking, user can still browse cached chat history)
41. WHEN a Peer's connection recovery fails (SDP negotiation timeout 15 seconds or ICE connection failure) THEN the client SHALL display a "Connection lost, click to retry" prompt in the corresponding chat session, allowing the user to manually trigger reconnection
42. WHEN all connection recovery completes THEN the client SHALL hide the recovery status prompt, UI returns to normal state
