# Requirement 5: End-to-End Encryption

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want my chat content and call data to be encrypted, so that my privacy is protected.

## Acceptance Criteria

### 5.1 One-on-One Encryption

1. WHEN two users establish a chat session THEN the system SHALL negotiate a shared key via the ECDH key exchange protocol for encrypting subsequent communication
2. WHEN a user sends a text message THEN the system SHALL encrypt the message content using AES-256-GCM before transmission
3. WHEN a user sends a file THEN the system SHALL encrypt file data in chunks before transmission
4. WHEN a WebRTC DataChannel is established THEN the system SHALL implement end-to-end encryption (E2EE) on top of the DataChannel, the signaling server cannot decrypt message content
5. IF key negotiation fails THEN the system SHALL notify the user that the encrypted channel establishment failed, and provide a retry option
6. WHEN a user views chat info THEN the system SHALL display an encryption status icon indicating whether the current session is encrypted

### 5.2 Multi-User Scenario Key Management

7. WHEN a multi-user chat/room has N participants THEN the system SHALL adopt a **Pairwise ECDH** strategy — each pair of Peers independently performs ECDH key exchange, establishing independent encrypted channels (i.e., N participants produce N*(N-1)/2 independent key pairs)
8. WHEN a new member joins a multi-user chat THEN the system SHALL perform ECDH key exchange between the new member and each existing member, establishing new pairwise encrypted channels; existing members' keys are unaffected
9. WHEN a member leaves a multi-user chat THEN the system SHALL destroy the pairwise keys between that member and all other members; remaining members' keys are unaffected
10. WHEN a user sends an encrypted message in multi-user chat THEN the system SHALL encrypt the message separately using the pairwise key with each target Peer, sending an independently encrypted message copy to each Peer

### 5.3 Theater Scenario Encryption Strategy

11. WHEN the theater owner distributes video/audio streams (MediaTrack) to viewers via WebRTC PeerConnection THEN the system SHALL **NOT apply application-layer E2EE**, relying on WebRTC's built-in DTLS-SRTP transport-layer encryption to protect media stream security (Reason: application-layer E2EE for media tracks requires `RTCRtpScriptTransform` / Insertable Streams API, which has limited browser compatibility and high implementation complexity, not in scope for this phase)
12. WHEN users send danmaku and chat messages via DataChannel in the theater THEN the system SHALL **NOT apply application-layer E2EE**, relying on WebRTC's built-in DTLS-SRTP transport-layer encryption to protect danmaku and message security (Reason: the theater uses star topology where the owner acts as a relay node needing to decrypt danmaku before forwarding to other viewers; Pairwise E2EE in this scenario means the owner is a trusted relay, which contradicts E2EE's "end-to-end" semantics; considering the theater is a public viewing scenario where danmaku content is not highly private, transport-layer encryption suffices for security needs)

### 5.4 Encryption Consistency

> **Architecture Note:** Since all Chat rooms and Theater rooms have a maximum of 8 participants (see Requirement 4.1), all messages are transmitted via DataChannel P2P with end-to-end encryption. There is no need for signaling server relay, ensuring a consistent security model across all rooms.
