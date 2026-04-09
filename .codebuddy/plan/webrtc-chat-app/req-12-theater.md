# Requirement 12: Shared Theater (Theater Mode)

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want to create a "theater" room, sharing local or online video with all viewers in the room for co-watching, with real-time danmaku and message interaction support, so that I can enjoy a shared viewing experience with friends.

## Acceptance Criteria

### 12.1 Copyright Notice & Disclaimer

0. WHEN a user creates a theater room or selects a video source THEN the system SHALL display a copyright notice: "Please ensure you have legal authorization to share this content. You are responsible for any copyright issues arising from content sharing." The notice SHALL be displayed in a non-intrusive manner (e.g., tooltip or info icon) and does not require user acknowledgment to proceed

### 12.2 Theater Room Management

1. WHEN a user clicks the "Create Theater" button THEN the system SHALL display a creation form, allowing setting theater name, description, password (optional); maximum viewer count is fixed at 8 (owner + 7 viewers)
2. WHEN a theater is created successfully THEN the system SHALL set the creator as the owner (Owner), the owner has full playback control and management permissions
3. WHEN a user browses the theater list THEN the system SHALL display theater name, owner name, current viewer count/max, playback status (idle/playing), whether encrypted, etc.
4. WHEN a user joins a theater THEN the system SHALL establish a WebRTC PeerConnection with the owner via the signaling server (star topology: owner is the central node, each viewer establishes only one PeerConnection with the owner), receiving the video stream distributed by the owner
4a. The system SHALL monitor the owner's resource usage (PeerConnection count, outbound bandwidth estimation via `RTCPeerConnection.getStats()`, and DataChannel message queue depth); WHEN the owner's estimated outbound bandwidth utilization exceeds 80% THEN the system SHALL display a "High load" warning on the owner side; WHEN the owner's DataChannel `bufferedAmount` consistently exceeds 1MB for more than 5 seconds THEN the system SHALL automatically reduce video stream quality — **specific degradation parameters**: resolution reduced from 1080p to 720p (or from 720p to 480p), frame rate reduced from 30fps to 15fps; **recovery threshold**: WHEN `bufferedAmount` drops below 512KB and stays below for more than 10 seconds THEN the system SHALL automatically attempt to restore higher quality (incrementally: first restore frame rate to 30fps, then after 10 seconds of stable buffer restore resolution); the system SHALL display "Auto-adjusted to 720p/15fps" or "Quality restored to 1080p/30fps" notification on the owner side during degradation and recovery

> **Topology Note:** The theater uses **Star Topology**, different from chat rooms' Mesh topology. The owner acts as the central node, maintaining an independent PeerConnection uplink to push video streams to each viewer. Viewers do not establish direct PeerConnections with each other. **Danmaku and message transport path**: Uses an **owner relay approach** — viewers send danmaku/messages via their DataChannel with the owner, the owner receives and forwards via respective DataChannels to all other viewers (maintaining pure P2P architecture, not going through the signaling server). The 8-person limit (owner + 7 viewers) means the owner needs to maintain up to 7 uplinks, which is a reasonable limit for a pure client-side approach.

5. IF a theater has a password set THEN the system SHALL require entering the correct password when a user joins
6. WHEN the owner actively leaves the theater THEN the system SHALL pause playback and notify all viewers "Owner has left", the owner can choose to transfer owner permissions to another viewer before leaving
6a. WHEN the owner unexpectedly disconnects (network fluctuation or other non-active leave scenario) THEN the system SHALL display "Owner connection interrupted, waiting for reconnection..." prompt on viewer side and pause playback; IF the owner reconnects within 30 seconds THEN the system SHALL automatically resume video stream distribution, viewer side automatically resumes playback; IF the owner does not reconnect within 30 seconds THEN the system SHALL notify all viewers "Owner has gone offline", and allow viewers to choose to wait or exit the theater
7. IF all users have left the theater THEN the system SHALL automatically destroy the theater and release resources
7a. WHEN the owner's CPU usage is excessively high (detected via frame drop rate in `requestAnimationFrame` callback — IF frame drop rate exceeds 30% for more than 10 seconds) THEN the system SHALL automatically reduce danmaku rendering density and video stream quality, and display "Performance degradation detected, automatically optimizing" on the owner side

### 12.3 Video Source Selection & Playback

8. WHEN the owner selects "Local Video" THEN the system SHALL open a file picker, supporting browser-natively-supported video formats (MP4, WebM, OGG), after selection load via `<video>` element on the owner side and capture MediaStream using `captureStream()` API; the system SHALL be compatible with both `captureStream()` and `mozCaptureStream()` APIs (Firefox uses `mozCaptureStream()`), and display "Your browser does not support video stream capture, please use Chrome or Edge" on unsupported browsers
9. WHEN the owner selects "Online Video" THEN the system SHALL display a URL input box, after the owner enters a direct video URL, the system loads via `<video>` element and captures MediaStream using `captureStream()`
10. IF the online video URL cannot be loaded due to CORS policy THEN the system SHALL prompt the user "This video source does not support cross-origin playback", and suggest downloading and using local video playback instead
11. WHEN the video source loads successfully THEN the owner side SHALL distribute the captured MediaStream (including video and audio tracks) to all connected viewers in the room via WebRTC PeerConnection
12. WHEN a new viewer joins a playing theater THEN the system SHALL automatically establish a PeerConnection for the new viewer and push the current video stream, the new viewer starts watching from the current playback position
13. The system SHALL support video formats including at least: MP4 (H.264), WebM (VP8/VP9), OGG (all browser `<video>` element natively supported formats)
14. IF the user selects a video format not supported by the browser (e.g., MKV, H.265, etc.) THEN the system SHALL prompt "This video format is not supported, please convert to MP4 (H.264) or WebM format and try again", and refuse to load

### 12.4 Playback Controls (Owner Only)

15. WHEN the owner clicks the play/pause button THEN the system SHALL control the local `<video>` element's playback state; since the video stream is captured in real-time, viewer side will automatically sync (stream freezes on pause, resumes on play)
16. WHEN the owner drags the progress bar THEN the system SHALL adjust the local `<video>` element's `currentTime`, the video stream automatically continues capturing from the new position, while broadcasting current playback progress info to all viewers via DataChannel
17. WHEN the owner adjusts volume THEN the system SHALL only adjust the owner's local volume, not affecting viewer volume (viewers have independent volume control)
18. WHEN the owner switches video source (changes video file or URL) THEN the system SHALL stop the current video stream, load the new video source, re-capture MediaStream and replace video/audio tracks in all PeerConnections
19. The system SHALL display a playback control bar at the bottom of the playback interface, including: play/pause, progress bar, current time/total duration, volume control, subtitle toggle, fullscreen toggle
20. The system SHALL periodically (every 5 seconds) sync playback progress info to viewers via DataChannel, for UI display (viewer progress bar is read-only)

### 12.4a Subtitle Support

20a. WHEN the owner clicks the "Subtitle" button in the playback control bar THEN the system SHALL display a subtitle settings panel with options to: load a subtitle file, toggle subtitle visibility, and adjust subtitle appearance
20b. The system SHALL support loading external subtitle files in **SRT** and **WebVTT (.vtt)** formats; WHEN the owner selects a subtitle file via file picker THEN the system SHALL parse the subtitle file and overlay subtitle text on the video playback area
20c. WHEN a subtitle file is loaded THEN the system SHALL sync subtitle display with the video playback timeline; subtitles SHALL appear and disappear according to their timestamp entries in the subtitle file
20d. The system SHALL broadcast subtitle data to all viewers via DataChannel: WHEN the owner loads a subtitle file THEN the system SHALL send the parsed subtitle entries (as a `SubtitleData` DataChannel message) to all connected viewers; viewers SHALL render subtitles locally in sync with the playback progress received from the owner
20e. The system SHALL support subtitle appearance customization: font size (small/medium/large, default: medium), text color (white with black outline by default), background opacity (0%-80%, default: 40%), position (bottom/top, default: bottom)
20f. WHEN the owner seeks to a new playback position THEN the system SHALL immediately update the displayed subtitle to match the new position
20g. WHEN the owner removes or replaces the subtitle file THEN the system SHALL broadcast a subtitle clear command to all viewers, removing subtitle display
20h. IF the subtitle file format is invalid or cannot be parsed THEN the system SHALL display an error message: "Subtitle file format is not supported. Please use SRT or WebVTT format."

### 12.5 Danmaku System

21. WHEN a user enters danmaku text and sends in the theater THEN the system SHALL broadcast the danmaku message to all users in the room via DataChannel (P2P)
22. WHEN a danmaku message arrives at the client THEN the system SHALL render the danmaku text scrolling from right to left above the video display
23. WHEN a user sends danmaku THEN the system SHALL support setting danmaku color (preset color palette) and danmaku position (top fixed/bottom fixed/scrolling)
24. WHEN danmaku density is too high (more than 50 displayed simultaneously) THEN the system SHALL automatically reduce danmaku rendering density (skipping some danmaku), prioritizing the newest danmaku
25. WHEN a user clicks the "Close Danmaku" button THEN the system SHALL hide the danmaku rendering layer, but not affect danmaku message receiving and sending
26. WHEN a user clicks the "Danmaku Settings" button THEN the system SHALL display a danmaku settings panel, supporting adjustment of danmaku transparency (0%-100%), font size (small/medium/large), scroll speed (slow/medium/fast)
27. The system SHALL use Canvas or CSS animation to render danmaku, ensuring danmaku rendering does not affect video playback performance
28. The system SHALL ensure danmaku end-to-end latency (from sender clicking send to all viewers' screens rendering the danmaku) < 500ms (in LAN environment); **Danmaku batch merge strategy**: The owner side SHALL batch-merge danmaku forwarding — collecting a batch of danmaku messages every 50ms, merging into a single DataChannel message before batch-sending to each viewer, reducing DataChannel message send count; IF owner-side danmaku forwarding delay exceeds 1s (owner-side load too high) THEN the system SHALL automatically reduce danmaku forwarding frequency (max 20 danmaku forwarded per second, excess discarded by FIFO), and display "Danmaku load is high, some danmaku may be delayed" prompt on the owner side

> **Danmaku Message Reliability Note:** Danmaku messages use a "best-effort" delivery strategy, no ACK confirmation or resend mechanism needed. Danmaku is a real-time-priority scenario where losing a small number of danmaku is acceptable.
>
> **Danmaku/Message Behavior During Owner Disconnection:** Since danmaku and messages are relayed through the owner, danmaku and message functionality will pause during owner disconnection. Viewer side SHALL display "Message functionality temporarily unavailable (owner reconnecting)" in the danmaku input box, automatically resuming after owner reconnects.

### 12.6 Theater Message Interaction

29. WHEN a user is in the theater THEN the system SHALL display a message panel to the right of the video playback area (desktop) or below (mobile)
30. WHEN a user sends a text message in the message panel THEN the system SHALL broadcast the message to all users in the room via DataChannel, messages displayed as chat bubbles
31. WHEN there are new messages in the message panel THEN the system SHALL auto-scroll to the latest message, and display an unread message count badge when the panel is collapsed
32. The system SHALL display sender username, send time, and message content in messages

### 12.7 Owner Permission Management

33. WHEN the owner or an admin selects a user in the viewer list and clicks "Kick" THEN the system SHALL send a unified `KickMember` signaling message (see Requirement 8, Room Management section), disconnect that user's PeerConnection, remove them from the theater, and notify the user "You have been removed from the theater"
34. WHEN the owner or an admin selects a user in the viewer list and clicks "Mute" THEN the system SHALL send a unified `MuteMember` signaling message (see Requirement 8, Room Moderation section), prohibiting that user from sending danmaku and messages; the muted user's input box SHALL display "You have been muted" and be non-editable
35. WHEN the owner or an admin clicks "Unmute" THEN the system SHALL send a unified `UnmuteMember` signaling message and restore that user's speaking permissions
36. WHEN the owner clicks "Mute All" THEN the system SHALL prohibit all users except the owner from sending danmaku and messages
37. IF a kicked user attempts to rejoin the same theater THEN the system SHALL reject the join and prompt "You have been removed from this theater by the owner"
38. WHEN the owner is in the viewer list THEN the system SHALL display each viewer's username, online status, role (Admin/Member), whether muted, and provide kick/mute/unmute action buttons (visible only to users with sufficient permissions per Requirement 15.3)
39. WHEN the owner clicks "Transfer Ownership" THEN the system SHALL transfer owner permissions to the specified viewer, the original owner becomes a regular viewer

### 12.8 Theater UI Layout

40. WHEN a user enters the theater on desktop THEN the system SHALL display a left large-area video playback zone + right message/viewer panel layout
41. WHEN a user enters the theater on mobile THEN the system SHALL display an upper video playback zone + lower switchable message/viewer panel
42. WHEN a user clicks the fullscreen button THEN the system SHALL display the video playback zone in fullscreen, danmaku overlaid on top of the video, message panel hidden (can be summoned via gesture or button)
43. The system SHALL display theater name, current viewer count, and owner indicator at the top of the theater page
