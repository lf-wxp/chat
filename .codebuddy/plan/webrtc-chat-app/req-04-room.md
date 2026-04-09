# Requirement 4: Room System

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to create and manage chat rooms, so that I can organize different chat groups.

> **Architecture Note:** The room system is a unified container model supporting two room types: **Chat (Chat Room)** and **Theater (Theater Room)**. Chat rooms are for multi-user text/audio-video communication (Requirements 2, 3), Theater rooms are for shared video viewing (Requirement 13). Both types share the room's basic management logic (create, join, leave, destroy, password protection), but differ in features and permissions.

## Acceptance Criteria

1. WHEN a user creates a room THEN the system SHALL allow selecting room type (Chat / Theater) and setting room name, description, password (optional); Chat room maximum participant limit is fixed at 8; Theater room maximum participant limit is fixed at 8 (owner + 7 viewers)

> **Participant Limit Unified Definition:** Both Chat room and Theater room have a maximum of 8 participants. This limit is consistent with the Mesh topology limit for WebRTC connections (for Chat rooms) and Star topology uplink bandwidth considerations (for Theater rooms). The 8-person limit ensures all messages are transmitted via DataChannel P2P with end-to-end encryption, eliminating the complexity of dual transport paths and ensuring a consistent security model. The participant limit referenced in Requirement 3.10 follows this requirement's definition.

2. IF a room has a password set THEN the system SHALL require entering the correct password when a user joins, rejecting on incorrect password
3. WHEN the room creator invites other users THEN the system SHALL generate an invitation link or directly send an invitation notification to the target user
4. WHEN a user receives a room invitation THEN the system SHALL display an invitation notification, the user can choose to accept or decline
5. WHEN the room creator (owner) manages the room THEN the system SHALL provide management functions including kick member, transfer ownership, modify room info (including room name, description, and password)
5a. WHEN the owner modifies the room password THEN the system SHALL require the owner to enter the new password twice for confirmation; after successful modification, the system SHALL notify all currently online members in the room "Room password has been changed by the owner"; members who are offline will need to enter the new password when they next attempt to join the room
5b. WHEN the owner clears the room password (sets it to empty) THEN the system SHALL remove password protection from the room, allowing anyone to join without a password
6. WHEN a user joins a room THEN the system SHALL display the user in the room member list, and broadcast a join notification to other members in the room
7. WHEN a user leaves a room THEN the system SHALL remove the user from the member list, and broadcast a leave notification
8. IF all members have left the room THEN the system SHALL automatically destroy the room and release resources
9. WHEN a user browses the room list THEN the system SHALL display room name, description, current/max participants, whether encrypted, etc.

> **Room Data Persistence Strategy:** Room list and room state are maintained in server memory, clients obtain the latest list via signaling, no local persistence cache. After page refresh, the client receives the server-pushed `RoomListUpdate` message after successful TokenAuth to restore the room list (see Requirement 11.2.9).
