# Requirement 1: SDP Signaling Service Enhancement

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a developer, I want the signaling server to support multi-user WebRTC connection establishment and management, so that users can conduct multi-user audio/video calls and data transfer.

## Acceptance Criteria

1. WHEN a client initiates a multi-user call request THEN the signaling server SHALL forward the SDP Offer to all target clients in the room, and collect SDP Answers to return to the initiator
2. WHEN a new user joins an existing call room THEN the signaling server SHALL coordinate the new user to establish a PeerConnection with each existing member in the room (Mesh topology)
3. WHEN a user leaves a call THEN the signaling server SHALL notify all other members in the room to update connection status, and clean up the corresponding signaling session
4. WHEN an ICE Candidate is generated THEN the signaling server SHALL precisely forward the ICE Candidate to the corresponding target Peer
5. IF the signaling server detects a WebSocket connection drop THEN the signaling server SHALL automatically clean up all signaling sessions for that client, and notify related Peers
6. WHEN the signaling server starts THEN the signaling server SHALL support configuring STUN/TURN server addresses via environment variables or command-line parameters, and deliver ICE configuration to clients upon connection; the system SHALL include default public STUN server configuration (e.g., `stun:stun.l.google.com:19302`); TURN server is a **recommended configuration** (not required), but the system SHALL prompt the user "Connection failed, you may need to configure a TURN server to traverse strict NAT environments" when ICE connection fails
7. WHEN a client connects via WebSocket THEN the signaling server SHALL implement a heartbeat detection mechanism (Ping/Pong), disconnecting if no response within timeout
8. WHEN a WebSocket connection drops unexpectedly THEN the client SHALL automatically attempt reconnection (exponential backoff strategy), restoring session state after successful reconnection
9. WHEN a user successfully establishes a WebRTC PeerConnection THEN the client SHALL send a `PeerEstablished` message via the signaling server, and the server SHALL record the Peer relationship in both parties' active_peers lists; WHEN a PeerConnection closes THEN the client SHALL send a `PeerClosed` message, and the server SHALL clean up the corresponding record
10. WHEN a user successfully re-authenticates via TokenAuth (page refresh scenario) THEN the signaling server SHALL send an `ActivePeersList` message to the user, containing the list of Peer user IDs that had established connections before the refresh, so the client can rebuild WebRTC connections
