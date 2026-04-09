# Requirement 9: Online User Discovery & Connection Invitation

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to see which users are currently online, and establish chat connections by sending connection invitations, so that I can proactively initiate conversations with other users while the other party has the right to decide whether to accept.

## Acceptance Criteria

1. WHEN a user logs in successfully THEN the system SHALL display an "Online Users" panel in the sidebar, showing a real-time list of all currently logged-in users (including username, avatar, online status)
2. WHEN a new user logs in or a user goes offline THEN the signaling server SHALL broadcast user list change events to all online clients via WebSocket, and clients SHALL update the online user list in real time
3. WHEN a user clicks on a user in the online user list THEN the system SHALL display that user's brief info card (username, online duration, status signature), and provide a "Send Connection Invitation" button
4. WHEN a user clicks the "Send Connection Invitation" button THEN the system SHALL send a connection invitation message to the target user via the signaling server (including initiator info and optional note), the initiator's UI SHALL display "Invitation sent, waiting for response" status
5. WHEN the target user receives a connection invitation THEN the system SHALL display the invitation info as a popup or notification card (initiator username, avatar, note), providing "Accept" and "Decline" action buttons
6. WHEN the target user clicks "Accept" THEN the system SHALL establish a WebRTC PeerConnection between both parties (via signaling server SDP exchange), after successful connection both parties automatically enter the one-on-one chat interface
7. WHEN the target user clicks "Decline" THEN the system SHALL notify the initiator via the signaling server that the invitation was declined, the initiator's UI SHALL display "The other party has declined your invitation" prompt
8. IF the target user does not respond to the invitation within 60 seconds THEN the system SHALL automatically mark the invitation as timed out, and notify the initiator "Invitation has timed out"
9. IF the initiator has already sent an unprocessed invitation to a user THEN the system SHALL prevent duplicate invitation sending, the button displays as "Invitation Pending" in a non-clickable state
10. WHEN a user has already established a PeerConnection with another user THEN the system SHALL mark that user as "Connected" in the online user list, clicking goes directly to the chat interface rather than sending another invitation
11. WHEN a user is in the online user list THEN the system SHALL support searching/filtering online users by username
12. WHEN a user wants to invite multiple people to chat THEN the system SHALL support multi-selecting online users and batch sending connection invitations; all invitation responses go back to the signaling server, which manages room creation and member joining uniformly; WHEN at least one invitee accepts THEN the signaling server SHALL immediately create a multi-user chat room (initiator + accepted users), and associate the `room_id` with remaining pending invitations; subsequently accepting users SHALL obtain the `room_id` via the signaling server and automatically join the room (the system SHALL broadcast new member join notifications to members already in the room, see Requirement 4.6); declined or timed-out users do not affect the created room; IF all invitees decline or time out THEN the system SHALL notify the initiator "No one accepted the invitation, multi-user chat was not created"
13. IF two users simultaneously send connection invitations to each other (bidirectional invitation concurrency conflict) THEN the system SHALL automatically detect bidirectional invitations and merge them into a single connection establishment — the signaling server checks upon receiving an invitation whether a reverse pending invitation already exists, if so automatically merges both invitations and directly establishes the PeerConnection without both parties separately accepting
14. IF a user is in the middle of SDP negotiation (PeerConnection being established) THEN the system SHALL queue newly received connection invitations, processing the next invitation in the queue after the current SDP negotiation completes, avoiding resource contention and state confusion from concurrent SDP negotiations; the system SHALL display "Connection being established, please wait..." status in the UI for queued invitations

### 9.2 Blacklist Functionality

15. WHEN a user clicks the "Block User" button on another user's info card or in a chat session THEN the system SHALL add that user to the current user's blacklist, and display a "User has been blocked" confirmation message
16. WHEN a user is added to the blacklist THEN the system SHALL automatically disconnect any existing PeerConnection with that user (if any), and the blocked user's UI SHALL display "The other party has disconnected" (without revealing the block action)
17. WHEN a blocked user attempts to send a connection invitation to the user who blocked them THEN the client SHALL silently auto-decline the invitation after a random delay (30-60 seconds) to simulate normal timeout behavior (the signaling server still forwards the invitation, but the blocking client auto-responds with decline after the delay); this preserves the client-side-only blacklist design while preventing the blocked user from detecting the block action
18. WHEN a blocked user is in the online user list THEN the system SHALL display a "Blocked" indicator next to their username, and the "Send Connection Invitation" button SHALL be disabled with a tooltip "You have blocked this user"
19. WHEN a user clicks the "Unblock User" button in their blacklist management panel THEN the system SHALL remove that user from the blacklist, allowing normal connection invitations again
20. WHEN a user accesses the blacklist management panel THEN the system SHALL display a list of all blocked users (username, block time), with an "Unblock" button next to each user
21. The blacklist data SHALL be stored in localStorage on the client side (not synchronized with the server), maintaining user privacy about their block actions
22. WHEN a user logs in on a different device or clears browser data THEN the blacklist data SHALL be reset to empty (since it's stored locally in localStorage)
