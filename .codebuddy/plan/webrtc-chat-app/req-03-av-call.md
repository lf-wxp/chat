# Requirement 3: Multi-User Audio/Video Calling

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to have voice or video calls with multiple people simultaneously, so that I can communicate in real time.

## Acceptance Criteria

1. WHEN a user initiates a video call in a room THEN the system SHALL send call invitations to all online members in the room, and establish WebRTC PeerConnections after they accept
2. WHEN a multi-user video call is in progress THEN the system SHALL display all participants' video streams in a Grid Layout, automatically adjusting layout based on participant count
3. WHEN a user clicks "Switch to Voice" during a video call THEN the system SHALL close the local camera video track, keeping only the audio track, and notify other participants to update UI (interaction control details see Requirement 7.1)
4. WHEN a user clicks "Enable Video" during a voice call THEN the system SHALL request camera permission, add a video track to the existing PeerConnection, and notify other participants (interaction control details see Requirement 7.2)
5. WHEN a user clicks the mute button THEN the system SHALL disable the local audio track, display a mute icon in the UI, and notify other participants that the user is muted
6. WHEN a user clicks the close camera button THEN the system SHALL disable the local video track, and other participants' UI SHALL display an avatar placeholder in that user's video area

> **Relationship with Requirement 8:** Requirement 3 focuses on multi-user call **establishment, topology management, and media stream negotiation** (e.g., Mesh topology connection establishment, participant join/leave, network adaptation). Requirement 8 focuses on **interaction controls and additional features** during calls (e.g., PiP floating window, incoming call notification, call duration statistics, message search). The two overlap on audio/video switching (3.3-3.4 and 8.1-8.2) and mute/close camera (3.5-3.6) — Requirement 3 defines underlying media track operations, Requirement 8 defines upper-layer UI interaction behavior.

7. WHEN someone is speaking during a call THEN the system SHALL detect audio activity (Voice Activity Detection) and highlight the current speaker in the UI
8. WHEN network quality degrades during a call THEN the system SHALL automatically reduce video resolution/frame rate to maintain call quality, and display a network quality indicator in the UI
8a. The system SHALL collect connection quality metrics via `RTCPeerConnection.getStats()` every 5 seconds for each active PeerConnection, extracting: round-trip time (RTT), packet loss rate, estimated available bandwidth (from `RTCIceCandidatePairStats`), and connection type (direct P2P vs. TURN relay)
8b. The system SHALL classify network quality into 4 levels based on collected metrics: **Excellent** (RTT < 100ms AND loss < 1%), **Good** (RTT < 200ms AND loss < 3%), **Fair** (RTT < 400ms AND loss < 8%), **Poor** (RTT ≥ 400ms OR loss ≥ 8%); the classification SHALL be displayed via the network quality indicator UI (see Requirement 14.10)
8c. WHEN network quality drops to "Fair" THEN the system SHALL automatically reduce video resolution from 720p to 480p and frame rate from 30fps to 15fps; WHEN quality drops to "Poor" THEN the system SHALL further reduce to 360p at 10fps; WHEN quality recovers to "Good" or "Excellent" for more than 10 consecutive seconds THEN the system SHALL incrementally restore quality (first frame rate, then resolution)
8d. The system SHALL store the latest network quality metrics in a Leptos Signal (`network_quality: RwSignal<HashMap<UserId, NetworkQuality>>`) so that UI components (video tiles, call controls) can reactively display per-peer quality indicators
9. WHEN a user is in a call THEN the system SHALL support screen sharing, with the shared screen displayed as a large view and other participants' videos arranged smaller
10. IF the room participant count reaches the Mesh topology limit (8 people, see Requirement 4.1 for participant limit definition) THEN the system SHALL prevent new users from joining the video call; the system SHALL prompt in the UI "Video call is at capacity (max 8 participants)"

> **Note:** Chat rooms have a maximum of 8 people (Requirement 4.1), consistent with the Mesh topology limit. This means all members in a Chat room can participate in video calls. The 8-person limit ensures all messages (text, voice, image) are transmitted via DataChannel P2P with end-to-end encryption, simplifying architecture and guaranteeing security.
