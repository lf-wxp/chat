# Requirement 15: User Profile Management & Unified Room Permission System

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to set my nickname and manage room permissions effectively across all room types (Chat & Theater), so that I can have a personalized identity and better organize group communications with consistent governance.

> **Architecture Note:** This requirement enhances user identity management and establishes a **unified room governance system** that applies to **all room types** (Chat Rooms and Theater Rooms as defined in Requirement 4). It includes four main features: (1) User nickname customization, (2) Room announcement management, (3) Unified administrator role with moderation powers for both Chat and Theater rooms, and (4) Room member search functionality. These features integrate with the existing room system (Requirement 4) and user authentication system (Requirement 10).

## Acceptance Criteria

### 15.1 User Nickname Management

1. WHEN a user registers THEN the system SHALL use the username as the default nickname
2. WHEN a user wants to change their display name THEN the system SHALL allow the user to set a custom nickname (distinct from username), supporting Chinese/English letters, numbers, underscores, and spaces, with a maximum length of 20 characters
3. IF a user sets a nickname containing prohibited words or special characters THEN the system SHALL reject the change and display an error message explaining the nickname policy
4. WHEN a user successfully changes their nickname THEN the system SHALL broadcast the nickname change to all online users via the signaling server, and all peer users' chat interfaces SHALL update the displayed nickname in real-time
5. WHEN a user views a chat message THEN the system SHALL display the sender's current nickname (or username if no nickname set), along with a small badge showing the original username for disambiguation
6. WHEN a user sets a nickname that is already in use by another online user THEN the system SHALL allow duplicate nicknames but display the username in parentheses to avoid confusion (e.g., "张三 (user_abc123)")
7. The system SHALL persist nickname changes to localStorage and restore them after page refresh (integrated with Requirement 10.9 client state persistence strategy)
8. WHEN a user clears their nickname (sets it to empty) THEN the system SHALL revert to displaying the username as the default identifier

### 15.2 Room Announcement Management

9. WHEN a room is created THEN the system SHALL initialize an empty announcement field for the room
10. WHEN the room owner creates or updates the room announcement THEN the system SHALL allow rich text formatting (bold, italic, links) with a maximum length of 500 characters, and display a preview before saving
11. WHEN a room has an announcement set THEN the system SHALL display the announcement prominently at the top of the chat area (collapsible panel), and highlight it with a different background color
12. WHEN the room owner updates the announcement THEN the system SHALL broadcast the new announcement to all current room members via signaling server, and their UIs SHALL update the announcement display in real-time
13. WHEN a new member joins a room with an existing announcement THEN the system SHALL automatically display the announcement to the new member upon successful join
14. WHEN the room owner clears the announcement THEN the system SHALL remove the announcement panel from all room members' chat interfaces
15. IF a room has no announcement set THEN the system SHALL hide the announcement panel completely (no empty placeholder)
16. WHEN the room owner edits the announcement THEN the system SHALL display an announcement edit modal with a character counter, formatting toolbar, and preview section
17. The system SHALL persist the room announcement in server memory (alongside room metadata) and include it in the RoomListUpdate signaling message

### 15.3 Unified Administrator Role & Moderation Powers (Applies to All Room Types)

> **Unified Permission Model:** The role and permission system defined in this section applies to **both Chat Rooms and Theater Rooms** uniformly. There is no distinction in permission granularity between room types—the same role hierarchy (Owner/Admin/Member) and moderation capabilities (kick/mute/ban) apply to all rooms in the system.

18. WHEN a room is created (Chat or Theater) THEN the system SHALL assign the creator as the room owner with full administrative powers
19. WHEN the room owner promotes a member to administrator in any room type (Chat or Theater) THEN the system SHALL broadcast the role change to all room members, and update the promoted user's permissions immediately
20. WHEN a room owner or administrator manages room members THEN the system SHALL provide the following moderation capabilities:
    - **Kick Member**: Remove a member from the room (owner and admins can kick regular members; only owner can kick admins)
    - **Mute Member**: Temporarily disable a member's ability to send chat messages for a specified duration (1 minute, 5 minutes, 30 minutes, 1 hour, permanent mute)
    - **Unmute Member**: Revoke a mute status early (owner and admins can unmute)
    - **Ban Member**: Prevent a user from rejoining the room (owner only)
    - **Unban Member**: Revoke a ban status (owner only)
21. WHEN a muted member attempts to send a message THEN the system SHALL reject the message and display a "You are muted until {end_time}" error message
22. WHEN a muted member's mute duration expires THEN the system SHALL automatically revoke the mute status and notify the member "Your mute has been lifted"
23. WHEN an administrator or owner kicks a member THEN the system SHALL broadcast a "{username} has been kicked by {admin_name}" notification to all remaining room members (excluding the kicked user)
24. WHEN an administrator or owner mutes a member THEN the system SHALL broadcast a "{username} has been muted for {duration} by {admin_name}" notification to all room members
25. WHEN a member is banned THEN the system SHALL add the member's user_id to the room's ban list (persisted in server memory); IF a banned user attempts to rejoin THEN the system SHALL reject the join request with a "You have been banned from this room" error message
26. WHEN a room owner demotes an administrator to regular member THEN the system SHALL broadcast the role change to all room members
27. WHEN a room owner transfers ownership to another member THEN the system SHALL demote the current owner to administrator role and promote the target member to owner; the new owner SHALL have full administrative powers
28. The system SHALL enforce permission checks on all moderation actions:
    - Regular members: Cannot perform any moderation actions
    - Administrators: Can kick/mute regular members, cannot kick/mute other admins or owner
    - Owner: Can perform all moderation actions including promoting/demoting admins and transferring ownership
29. WHEN a room owner leaves the room THEN the system SHALL automatically transfer ownership to the longest-serving administrator (or the longest-joined member if no admins exist)
30. IF a room has no members left after the owner leaves THEN the system SHALL destroy the room as per Requirement 4.8

### 15.4 Room Member Search

31. WHEN a user is in a room THEN the system SHALL provide a member search input field in the member list panel
32. WHEN a user types in the member search field THEN the system SHALL filter the member list in real-time based on nickname or username (case-insensitive partial match)
33. WHEN the member list exceeds 20 members THEN the system SHALL display a scrollable member list with virtual scrolling for performance optimization (similar to Requirement 14 virtual scrolling for message lists)
34. WHEN a search query matches one or more members THEN the system SHALL highlight the matching members in the list and display a count of results (e.g., "3 results found")
35. WHEN a user clicks on a member in the search results THEN the system SHALL open a context menu allowing: view profile, start 1-on-1 chat, mention (@username), or (if user has admin powers) moderation actions (mute/kick/ban)
36. WHEN a search query yields no results THEN the system SHALL display a "No members found matching '{query}'" message
37. WHEN the member search field is cleared THEN the system SHALL restore the full member list sorted by role (owner first, then admins, then regular members) and join time
38. The system SHALL optimize member search performance by maintaining an in-memory index of member nicknames and usernames on the client side (rebuilt on member join/leave events)

### 15.5 Integration with Existing Systems

39. WHEN a user's nickname changes THEN the system SHALL update the nickname in all active chat sessions, room member lists, and the online user list without requiring page refresh (real-time sync via signaling)
40. WHEN a member is kicked or banned from a room THEN the system SHALL close the corresponding WebRTC PeerConnection between the kicked/banned user and other room members (triggering `PeerClosed` signaling as per Requirement 10.11)
41. WHEN a member is muted THEN the system SHALL update the mute status in server memory and broadcast the mute event via signaling; the muted user's client SHALL disable message input UI elements locally
42. WHEN a room announcement is updated THEN the system SHALL persist the announcement in server memory and include it in the `RoomInfo` structure returned by `JoinRoom` signaling
43. The system SHALL display member role badges in the member list (Owner: 👑, Admin: ⭐, Member: no badge) and in chat messages (small icon next to nickname)
44. The system SHALL support batch moderation actions for administrators (e.g., select multiple members and mute all at once) with a maximum batch size of 5 members per operation to prevent abuse
45. The system SHALL log all moderation actions (kick/mute/ban/promote/demote) in the server's structured logs (via `tracing` crate) with fields: action_type, room_id, admin_id, target_user_id, duration (if applicable), timestamp

### 15.6 User Experience & Accessibility

46. WHEN a user is muted or banned THEN the system SHALL display a clear, non-intrusive notification explaining the action and duration (if temporary)
47. WHEN an administrator performs a moderation action THEN the system SHALL require a confirmation dialog for destructive actions (kick, ban, permanent mute) to prevent accidental clicks
48. The system SHALL ensure all moderation action buttons are keyboard-accessible with appropriate `aria-label` attributes (e.g., "Kick user {username}", "Mute user {username} for 5 minutes")
49. WHEN a screen reader user navigates the member list THEN the system SHALL announce member roles and status (e.g., "{username}, administrator, currently muted")
50. The system SHALL provide a "Member History" feature for room owners and admins, showing a chronological list of moderation actions taken in the room (stored in server memory, limited to last 100 actions per room)

---

## Implementation Notes

- **Unified Permission Architecture**: The role and permission system is designed as a **single, reusable permission module** shared by all room types (Chat Rooms and Theater Rooms). The server maintains a unified `RoomRole` enum (Owner, Admin, Member) and permission checking logic that applies identically to both room types.
- **Nickname Storage**: Nicknames are stored in server memory (`DashMap<UserId, UserProfile>` where `UserProfile` contains `username`, `nickname`, `avatar`, `status`) and persisted to client localStorage for offline recovery
- **Room Metadata Extension**: Extend the `Room` structure in server memory to include `announcement: Option<String>`, `admins: HashSet<UserId>`, `banned_users: HashSet<UserId>`, `muted_users: HashMap<UserId, MuteInfo>` where `MuteInfo` contains `end_time: Option<DateTime<Utc>>` (None = permanent). **This metadata structure is identical for both Chat and Theater rooms.**
- **Permission Middleware**: All moderation signaling messages (Kick, Mute, Ban, Promote, Demote) SHALL pass through a server-side permission middleware that verifies the action issuer has sufficient privileges before executing the action. **The middleware logic is room-type agnostic.**
- **Real-time Sync**: All nickname changes, role changes, and moderation actions SHALL trigger signaling broadcasts to affected users immediately to ensure UI consistency across all clients
- **Scalability Consideration**: Member search uses client-side in-memory filtering (suitable for ≤8 participants per room); for future scalability to larger rooms, consider server-side search with pagination

---

## Dependencies

- **Requires**: Requirement 4 (Room System), Requirement 10 (User Auth & Session Management)
- **Integrates with**: Requirement 2 (Chat System) for message input disable when muted, Requirement 11 (Persistence) for nickname persistence, Requirement 14 (UI Interaction) for moderation UI components
