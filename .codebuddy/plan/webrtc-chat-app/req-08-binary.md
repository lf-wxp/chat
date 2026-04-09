# Requirement 8: Full-Link Binary Transport

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a developer, I want all data transport to use binary format, so that transport efficiency is improved and bandwidth consumption is reduced.

## Acceptance Criteria

1. WHEN the client communicates with the signaling server THEN the system SHALL use bitcode binary serialization format for all WebSocket signaling messages (WebSocket only carries signaling, not chat messages or file data)
2. WHEN DataChannel transports chat messages and file data THEN the system SHALL use binary format (ArrayBuffer), reusing the message crate's bitcode serialization
3. WHEN transporting file data THEN the system SHALL use an efficient binary chunking protocol (dynamically adjustable chunk size, supporting flow control and resume transfer, tracking transfer progress via chunk bitmap)
4. WHEN transporting large messages (exceeding single DataChannel message size limit) THEN the system SHALL automatically perform chunked transfer, with the receiver automatically reassembling; the system SHALL adopt a conservative **64KB** as the maximum single DataChannel message size limit (balancing Chrome ~256KB and Firefox ~1MB different SCTP implementations, taking the conservative value for cross-browser compatibility)
5. The system SHALL define bitcode serialization/deserialization implementations for all message types (text, Sticker, voice, image, file, danmaku, system notification, etc.) in the message crate, ensuring frontend-backend protocol consistency
6. The system SHALL define independent binary encoding formats for each message type: text messages carry UTF-8 content; Sticker messages carry Sticker pack ID + Sticker ID (resources loaded on demand); voice messages carry Opus-encoded audio data + duration metadata; image messages carry thumbnail binary data + original image metadata (width, height, size, format)

## Signaling Message Type Catalog

The following is the unified catalog of all signaling message types exchanged over WebSocket between client and server. All messages use bitcode binary serialization via the shared `message` crate.

### Connection & Authentication
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `TokenAuth` | Client → Server | `{ token: String }` | JWT authentication on WebSocket connect / page refresh |
| `AuthSuccess` | Server → Client | `{ user_id: UserId, username: String }` | Authentication success response |
| `AuthFailure` | Server → Client | `{ reason: String }` | Authentication failure response |
| `ErrorResponse` | Server → Client | `{ code: String, message: String, i18n_key: String, details: Option<JsonObject>, timestamp: u64, trace_id: String }` | **Unified error response** (see [requirements.md](./requirements.md#error-code-specification) for error code registry) |
| `UserLogout` | Client → Server | `{}` | Active logout notification |
| `Ping` | Bidirectional | `{}` | Heartbeat ping |
| `Pong` | Bidirectional | `{}` | Heartbeat pong |

### User Discovery & Status
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `UserListUpdate` | Server → Client | `{ users: Vec<UserInfo> }` | Full/incremental online user list update |
| `UserStatusChange` | Server → Client | `{ user_id: UserId, status: UserStatus, signature: Option<String> }` | User status/signature change broadcast |

### Connection Invitation
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `ConnectionInvite` | Client → Server → Client | `{ from: UserId, to: UserId, note: Option<String> }` | Connection invitation |
| `InviteAccepted` | Client → Server → Client | `{ from: UserId, to: UserId }` | Invitation accepted |
| `InviteDeclined` | Client → Server → Client | `{ from: UserId, to: UserId }` | Invitation declined |
| `InviteTimeout` | Server → Client | `{ from: UserId, to: UserId }` | Invitation timed out |
| `MultiInvite` | Client → Server | `{ from: UserId, targets: Vec<UserId> }` | Multi-user invitation |

### SDP / ICE Signaling
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `SdpOffer` | Client → Server → Client | `{ from: UserId, to: UserId, sdp: String }` | SDP Offer forwarding |
| `SdpAnswer` | Client → Server → Client | `{ from: UserId, to: UserId, sdp: String }` | SDP Answer forwarding |
| `IceCandidate` | Client → Server → Client | `{ from: UserId, to: UserId, candidate: String }` | ICE Candidate forwarding |

### Peer Tracking
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `PeerEstablished` | Client → Server | `{ from: UserId, to: UserId }` | PeerConnection established notification |
| `PeerClosed` | Client → Server | `{ from: UserId, to: UserId }` | PeerConnection closed notification |
| `ActivePeersList` | Server → Client | `{ peers: Vec<UserId> }` | Active peers list (for refresh recovery) |

### Room Management
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `CreateRoom` | Client → Server | `{ name: String, room_type: RoomType, password: Option<String>, max_participants: u8 }` | Create room |
| `JoinRoom` | Client → Server | `{ room_id: RoomId, password: Option<String> }` | Join room |
| `LeaveRoom` | Client → Server | `{ room_id: RoomId }` | Leave room |
| `RoomListUpdate` | Server → Client | `{ rooms: Vec<RoomInfo> }` | Room list update |
| `RoomMemberUpdate` | Server → Client | `{ room_id: RoomId, members: Vec<MemberInfo> }` | Room member list update |
| `KickMember` | Client → Server | `{ room_id: RoomId, target: UserId }` | Kick member from room |
| `TransferOwnership` | Client → Server | `{ room_id: RoomId, target: UserId }` | Transfer room ownership |

### Call Signaling
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `CallInvite` | Client → Server → Client | `{ room_id: RoomId, media_type: MediaType }` | Call invitation |
| `CallAccept` | Client → Server → Client | `{ room_id: RoomId }` | Accept call |
| `CallDecline` | Client → Server → Client | `{ room_id: RoomId }` | Decline call |
| `CallEnd` | Client → Server → Client | `{ room_id: RoomId }` | End call |

### Theater Signaling
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `TheaterMuteAll` | Client → Server → Client | `{ room_id: RoomId }` | Mute all viewers |
| `TheaterTransferOwner` | Client → Server → Client | `{ room_id: RoomId, target: UserId }` | Transfer theater ownership |

### Room Moderation & Profile (Unified for All Room Types)
| Message Type | Direction | Payload | Description |
|---|---|---|---|
| `MuteMember` | Client → Server → Client | `{ room_id: RoomId, target: UserId, duration_secs: Option<u64> }` | Mute a member in any room type (replaces Theater-specific TheaterMute) |
| `UnmuteMember` | Client → Server → Client | `{ room_id: RoomId, target: UserId }` | Unmute a member in any room type (replaces Theater-specific TheaterUnmute) |
| `BanMember` | Client → Server | `{ room_id: RoomId, target: UserId }` | Ban a member from a room (kicked + cannot rejoin) |
| `UnbanMember` | Client → Server | `{ room_id: RoomId, target: UserId }` | Unban a member from a room |
| `PromoteAdmin` | Client → Server → Client | `{ room_id: RoomId, target: UserId }` | Promote a member to Admin role |
| `DemoteAdmin` | Client → Server → Client | `{ room_id: RoomId, target: UserId }` | Demote an Admin back to Member role |
| `NicknameChange` | Client → Server → Client | `{ user_id: UserId, new_nickname: String }` | User nickname change broadcast |
| `RoomAnnouncement` | Client → Server → Client | `{ room_id: RoomId, content: String }` | Room announcement update broadcast |
| `ModerationNotification` | Server → Client | `{ room_id: RoomId, action: String, target: UserId, actor: UserId, reason: Option<String> }` | Notification of moderation action to room members |

### DataChannel Message Types (P2P, not via signaling server)
| Message Type | Transport | Payload | Description |
|---|---|---|---|
| `ChatText` | DataChannel | `{ message_id: Uuid, content: String, timestamp: u64 }` | Text message |
| `ChatSticker` | DataChannel | `{ message_id: Uuid, pack_id: String, sticker_id: String }` | Sticker message |
| `ChatVoice` | DataChannel | `{ message_id: Uuid, opus_data: Vec<u8>, duration_ms: u32 }` | Voice message |
| `ChatImage` | DataChannel | `{ message_id: Uuid, thumbnail: Vec<u8>, metadata: ImageMeta }` | Image message |
| `FileChunk` | DataChannel | `{ transfer_id: Uuid, chunk_index: u32, data: Vec<u8> }` | File transfer chunk |
| `FileMetadata` | DataChannel | `{ transfer_id: Uuid, filename: String, size: u64, sha256: String, total_chunks: u32 }` | File transfer metadata |
| `MessageAck` | DataChannel | `{ message_id: Uuid }` | Message acknowledgment |
| `MessageRevoke` | DataChannel | `{ message_id: Uuid }` | Message revoke command |
| `TypingIndicator` | DataChannel | `{ is_typing: bool }` | Typing status |
| `EcdhKeyExchange` | DataChannel | `{ public_key: Vec<u8> }` | ECDH key exchange |
| `AvatarRequest` | DataChannel | `{ user_id: UserId }` | Request avatar data |
| `AvatarData` | DataChannel | `{ user_id: UserId, data: Vec<u8>, hash: String }` | Avatar data response |
| `MessageRead` | DataChannel | `{ message_ids: Vec<Uuid> }` | Read receipt (batch, sent when messages scroll into viewport) |
| `ForwardMessage` | DataChannel | `{ message_id: Uuid, original_message_id: Uuid, original_sender_id: UserId, original_sender_name: String, original_timestamp: u64, content_type: MessageContentType, content: Vec<u8> }` | Forwarded message (carries original message metadata + content) |
| `MessageReaction` | DataChannel | `{ target_message_id: Uuid, emoji: String, action: ReactionAction, reactor_id: UserId }` | Message reaction add/remove (emoji reaction on a message) |
| `Danmaku` | DataChannel | `{ content: String, color: String, position: DanmakuPosition }` | Theater danmaku |
| `PlaybackProgress` | DataChannel | `{ current_time: f64, duration: f64 }` | Theater playback progress sync |
| `SubtitleData` | DataChannel | `{ entries: Vec<SubtitleEntry> }` | Theater subtitle data (parsed SRT/VTT entries sent from owner to viewers) |
| `SubtitleClear` | DataChannel | `{}` | Theater subtitle clear command (owner removed/replaced subtitle file) |

> **Note:** The exact Rust enum/struct definitions are maintained in the shared `message` crate. The above table serves as a reference catalog for all message types across the system. Payload descriptions are simplified; actual implementations include additional fields (e.g., `sender_id`, `timestamp`, encryption metadata) as defined in the crate.

## Binary Protocol Specification

This section details the binary encoding format for all message types. The system uses **bitcode** serialization (a compact binary format) with the following conventions:

### General Encoding Rules

1. **Byte Order**: All multi-byte integers use **Little-Endian** encoding
2. **Variable-Length Integers**: Use `varint` encoding for `u64`, `i64`, `usize` (similar to protobuf varint)
3. **Strings**: Encoded as `length: varint` + `UTF-8 bytes`
4. **Vectors/Arrays**: Encoded as `length: varint` + `element encoding...`
5. **Option<T>**: Encoded as `flag: u8` (0=None, 1=Some) + `T encoding if Some`
6. **UUID**: Encoded as 16 bytes (big-endian, standard UUID format)
7. **Message Type Discriminator**: First byte indicates message type (enum variant index)

### Message Frame Structure

All WebSocket and DataChannel messages follow this frame structure:

```
┌──────────────┬──────────────┬─────────────────────────────┐
│ Magic Number │ Message Type │     Payload (Variable)      │
│   (2 bytes)  │   (1 byte)   │   (Depends on message)      │
└──────────────┴──────────────┴─────────────────────────────┘
```

- **Magic Number**: `0xBC` `0xBC` (constant, identifies bitcode protocol)
- **Message Type**: Enum discriminator (0-255, see message type mapping below)
- **Payload**: Bitcode-serialized message content

### Primitive Type Encoding

#### Integer Types

```
u8 / i8:    [value]                           (1 byte)
u16 / i16:  [value_low] [value_high]          (2 bytes, little-endian)
u32 / i32:  [value_byte0] ... [value_byte3]   (4 bytes, little-endian)
u64 / i64:  varint encoding                   (1-9 bytes, see varint spec)
usize:      varint encoding                   (1-9 bytes, same as u64)
```

#### Varint Encoding (Variable-Length Integer)

```
┌─────────────────────────────────────────────────────────────┐
│  Each byte: [MSB=continuation] [7-bit payload]              │
│  - MSB=1: more bytes follow                                 │
│  - MSB=0: last byte                                         │
│  Example: value 300                                         │
│    = 0xAC 0x02  (binary: 10101100 00000010)                │
│    = (0x2C << 0) | (0x02 << 7) = 300                        │
└─────────────────────────────────────────────────────────────┘
```

#### String Encoding

```
┌──────────────┬─────────────────────────────────┐
│ Length       │ UTF-8 Bytes                     │
│ (varint)     │ (Length bytes)                  │
└──────────────┴─────────────────────────────────┘

Example: "Hello" (5 bytes)
  = 0x05 0x48 0x65 0x6C 0x6C 0x6F
  = [5] ['H'] ['e'] ['l'] ['l'] ['o']
```

#### Option<T> Encoding

```
┌──────────────┬─────────────────────────────────┐
│ Flag         │ Value (if Flag=1)               │
│ (u8: 0/1)    │ (T encoding)                    │
└──────────────┴─────────────────────────────────┘

Example: Some("test")
  = 0x01 0x04 0x74 0x65 0x73 0x74
  = [1] [4] ['t'] ['e'] ['s'] ['t']

Example: None
  = 0x00
```

#### Vec<T> Encoding

```
┌──────────────┬─────────────────────────────────┐
│ Length       │ Elements                        │
│ (varint)     │ (T encoding repeated)           │
└──────────────┴─────────────────────────────────┘

Example: Vec<u8> = [1, 2, 3]
  = 0x03 0x01 0x02 0x03
  = [3] [1] [2] [3]
```

#### UUID Encoding

```
┌────────────────────────────────────────────────┐
│ 16 Bytes (Big-Endian, RFC 4122 format)         │
└────────────────────────────────────────────────┘

Example: 550e8400-e29b-41d4-a716-446655440000
  = [0x55 0x0E 0x84 0x00 0xE2 0x9B 0x41 0xD4 
     0xA7 0x16 0x44 0x66 0x55 0x44 0x00 0x00]
```

### Message Type Mapping (Discriminator Values)

#### Signaling Messages (WebSocket)

| Discriminator | Message Type |
|---|---|
| 0x00 | TokenAuth |
| 0x01 | AuthSuccess |
| 0x02 | AuthFailure |
| 0x06 | ErrorResponse |
| 0x03 | UserLogout |
| 0x04 | Ping |
| 0x05 | Pong |
| 0x10 | UserListUpdate |
| 0x11 | UserStatusChange |
| 0x20 | ConnectionInvite |
| 0x21 | InviteAccepted |
| 0x22 | InviteDeclined |
| 0x23 | InviteTimeout |
| 0x24 | MultiInvite |
| 0x30 | SdpOffer |
| 0x31 | SdpAnswer |
| 0x32 | IceCandidate |
| 0x40 | PeerEstablished |
| 0x41 | PeerClosed |
| 0x42 | ActivePeersList |
| 0x50 | CreateRoom |
| 0x51 | JoinRoom |
| 0x52 | LeaveRoom |
| 0x53 | RoomListUpdate |
| 0x54 | RoomMemberUpdate |
| 0x55 | KickMember |
| 0x56 | TransferOwnership |
| 0x60 | CallInvite |
| 0x61 | CallAccept |
| 0x62 | CallDecline |
| 0x63 | CallEnd |
| 0x70 | TheaterMuteAll |
| 0x71 | TheaterTransferOwner |
| 0x75 | MuteMember |
| 0x76 | UnmuteMember |
| 0x77 | BanMember |
| 0x78 | UnbanMember |
| 0x79 | PromoteAdmin |
| 0x7A | DemoteAdmin |
| 0x7B | NicknameChange |
| 0x7C | RoomAnnouncement |
| 0x7D | ModerationNotification |

#### DataChannel Messages (P2P)

| Discriminator | Message Type |
|---|---|
| 0x80 | ChatText |
| 0x81 | ChatSticker |
| 0x82 | ChatVoice |
| 0x83 | ChatImage |
| 0x90 | FileChunk |
| 0x91 | FileMetadata |
| 0xA0 | MessageAck |
| 0xA1 | MessageRevoke |
| 0xA2 | TypingIndicator |
| 0xA3 | MessageRead |
| 0xA4 | ForwardMessage |
| 0xA5 | MessageReaction |
| 0xB0 | EcdhKeyExchange |
| 0xB1 | AvatarRequest |
| 0xB2 | AvatarData |
| 0xC0 | Danmaku |
| 0xC1 | PlaybackProgress |
| 0xC2 | SubtitleData |
| 0xC3 | SubtitleClear |

### Detailed Binary Layout Examples

#### Example 1: TokenAuth Message

**Rust Struct:**
```rust
struct TokenAuth {
    token: String,
}
```

**Binary Layout:**
```
┌──────────┬──────────┬─────────────┬──────────────────────┐
│ Magic    │ Type     │ Token Len   │ Token UTF-8          │
│ 2 bytes  │ 1 byte   │ varint      │ (variable)           │
└──────────┴──────────┴─────────────┴──────────────────────┘

Example: TokenAuth { token: "abc123" }
Hex: BC BC 00 06 61 62 63 31 32 33
     │  │  │  └───────────────────── Token: "abc123"
     │  │  └──────────────────────── Token length: 6
     │  └─────────────────────────── Message type: 0x00 (TokenAuth)
     └────────────────────────────── Magic number: 0xBCBC
```

#### Example 2: AuthSuccess Message

**Rust Struct:**
```rust
struct AuthSuccess {
    user_id: UserId,    // u64
    username: String,
}
```

**Binary Layout:**
```
┌──────────┬──────────┬──────────┬─────────────┬──────────────────┐
│ Magic    │ Type     │ User ID  │ Username Len│ Username UTF-8   │
│ 2 bytes  │ 1 byte   │ varint   │ varint      │ (variable)       │
└──────────┴──────────┴──────────┴─────────────┴──────────────────┘

Example: AuthSuccess { user_id: 12345, username: "alice" }
Hex: BC BC 01 B9 60 05 61 6C 69 63 65
     │  │  │  │  └─────────────────── Username: "alice"
     │  │  │  └───────────────────── Username length: 5
     │  │  └──────────────────────── User ID: 12345 (varint: 0xB9 0x60)
     │  └─────────────────────────── Message type: 0x01 (AuthSuccess)
     └────────────────────────────── Magic number: 0xBCBC
```

#### Example 3: ErrorResponse Message (Unified Error Protocol)

**Rust Struct:**
```rust
struct ErrorResponse {
    code: String,           // Error code from registry (e.g., "SIG003")
    message: String,        // Default English error message
    i18n_key: String,       // Key for localized message lookup
    details: Option<JsonValue>,  // Optional contextual details
    timestamp: u64,         // ISO 8601 timestamp (Unix epoch)
    trace_id: String,       // Unique trace identifier for logging
}
```

**Binary Layout:**
```
┌──────────┬──────────┬─────────────┬────────────────┬─────────────┬──────────────────┬─────────────┬──────────────┬─────────────┬────────────────┬─────────────┬────────────────┐
│ Magic    │ Type     │ Code Len    │ Code UTF-8     │ Msg Len     │ Message UTF-8    │ Key Len     │ i18n_key     │ Details Flag│ Details JSON   │ Timestamp   │ Trace ID Len   │
│ 2 bytes  │ 1 byte   │ varint      │ (variable)     │ varint      │ (variable)       │ varint      │ UTF-8        │ u8 (0/1)    │ (if flag=1)    │ varint      │ + Trace UTF-8  │
└──────────┴──────────┴─────────────┴────────────────┴─────────────┴──────────────────┴─────────────┴──────────────┴─────────────┴────────────────┴─────────────┴────────────────┘

Example: ErrorResponse { 
    code: "SIG003", 
    message: "ICE connection failed", 
    i18n_key: "error.sig003",
    details: Some({"retry_count": 2, "last_ice_state": "failed"}),
    timestamp: 1712603216,
    trace_id: "abc123def456"
}

Binary breakdown:
- Magic: BC BC
- Type: 06 (ErrorResponse)
- Code: 06 53 49 47 30 30 33 (length=6, "SIG003")
- Message: 14 49 43 45 20 63 6F 6E 6E 65 63 74 69 6F 6E 20 66 61 69 6C 65 64 (length=20, "ICE connection failed")
- i18n_key: 0D 65 72 72 6F 72 2E 73 69 67 30 30 33 (length=13, "error.sig003")
- Details flag: 01 (Some)
- Details JSON: 2B 7B 22 72 65 74 72 79 5F 63 6F 75 6E 74 22 3A 20 32 2C 20 22 6C 61 73 74 5F 69 63 65 5F 73 74 61 74 65 22 3A 20 22 66 61 69 6C 65 64 22 7D 
  (length=43, '{"retry_count": 2, "last_ice_state": "failed"}')
- Timestamp: 80 F8 B0 5D 06 (varint for 1712603216)
- Trace ID: 0C 61 62 63 31 32 33 64 65 66 34 35 36 (length=12, "abc123def456")

Hex (compact):
BC BC 06 06 53 49 47 30 30 33 14 49 43 45 20 63 6F 6E 6E 65 63 74 69 6F 6E 20 66 61 69 6C 65 64 
0D 65 72 72 6F 72 2E 73 69 67 30 30 33 01 2B 7B 22 72 65 74 72 79 5F 63 6F 75 6E 74 22 3A 20 
32 2C 20 22 6C 61 73 74 5F 69 63 65 5F 73 74 61 74 65 22 3A 20 22 66 61 69 6C 65 64 22 7D 
80 F8 B0 5D 06 0C 61 62 63 31 32 33 64 65 66 34 35 36
```

**Usage Context:**
- WHEN the server encounters an error (authentication failure, SDP negotiation timeout, ICE connection failed, etc.) THEN it SHALL send an `ErrorResponse` message to the client
- The client SHALL use `i18n_key` to look up the localized error message in `/assets/i18n/{locale}.json`
- The client SHALL display the error in the UI with an optional "Learn more" expandable section (see [requirements.md](./requirements.md#error-message-internationalization))
- The `trace_id` SHALL be included in both client and server logs for error tracing across the full request lifecycle

#### Example 3: ChatText Message (DataChannel)

**Rust Struct:**
```rust
struct ChatText {
    message_id: Uuid,    // 16 bytes
    content: String,
    timestamp: u64,      // varint
}
```

**Binary Layout:**
```
┌──────────┬──────────┬─────────────────────┬─────────────┬──────────────┬───────────┐
│ Magic    │ Type     │ Message ID (UUID)   │ Content Len │ Content UTF-8│ Timestamp │
│ 2 bytes  │ 1 byte   │ 16 bytes            │ varint      │ (variable)   │ varint    │
└──────────┴──────────┴─────────────────────┴─────────────┴──────────────┴───────────┘

Example: ChatText { 
    message_id: "550e8400-e29b-41d4-a716-446655440000", 
    content: "Hi", 
    timestamp: 1617181920 
}
Hex: BC BC 80 55 0E 84 00 E2 9B 41 D4 A7 16 44 66 55 44 00 00 02 48 69 80 F8 B0 5D 06
     │  │  └───────────────────────────────────────────────── └─ ─── └─────────────
     │  │                   Message ID (UUID)                 │   │   Timestamp
     │  │                                                    │   └─ Content: "Hi"
     │  │                                                    └─ Content length: 2
     │  └─ Message type: 0x80 (ChatText)
     └─ Magic number
```

#### Example 4: FileChunk Message (Large File Transfer)

**Rust Struct:**
```rust
struct FileChunk {
    transfer_id: Uuid,   // 16 bytes
    chunk_index: u32,    // 4 bytes (little-endian)
    data: Vec<u8>,       // variable
}
```

**Binary Layout:**
```
┌──────────┬──────────┬─────────────────────┬─────────────┬─────────────┬────────────────┐
│ Magic    │ Type     │ Transfer ID (UUID)  │ Chunk Index │ Data Length │ Data Bytes     │
│ 2 bytes  │ 1 byte   │ 16 bytes            │ 4 bytes LE  │ varint      │ (variable)     │
└──────────┴──────────┴─────────────────────┴─────────────┴─────────────┴────────────────┘

Example: FileChunk { 
    transfer_id: "...", 
    chunk_index: 42, 
    data: [0xFF, 0xAB, 0x00] 
}
Hex: BC BC 90 [UUID 16 bytes] 2A 00 00 00 03 FF AB 00
     │  │  │                   └───────────── └─ ─── ─── Data: [0xFF, 0xAB, 0x00]
     │  │  │                     │             └─ Data length: 3
     │  │  │                     └─ Chunk index: 42 (little-endian)
     │  │  └─ Transfer ID (UUID)
     │  └─ Message type: 0x90 (FileChunk)
     └─ Magic number
```

#### Example 5: Vec<UserInfo> in UserListUpdate

**Rust Struct:**
```rust
struct UserInfo {
    user_id: UserId,     // u64 (varint)
    username: String,
    status: UserStatus,  // u8 enum
    signature: Option<String>,
}

struct UserListUpdate {
    users: Vec<UserInfo>,
}
```

**Binary Layout:**
```
┌──────────┬──────────┬─────────────┬─────────────────────────────────────┐
│ Magic    │ Type     │ Users Count │ User Info Array                     │
│ 2 bytes  │ 1 byte   │ varint      │ (repeated UserInfo encoding)        │
└──────────┴──────────┴─────────────┴─────────────────────────────────────┘

UserInfo Encoding:
┌──────────┬─────────────┬──────────────┬────────────┬─────────────┬──────────────┐
│ User ID  │ Username Len│ Username     │ Status     │ Sig Flag    │ Signature    │
│ varint   │ varint      │ (variable)   │ u8         │ u8 (0/1)    │ (if flag=1)  │
└──────────┴─────────────┴──────────────┴────────────┴─────────────┴──────────────┘

Example: UserListUpdate { 
    users: [
        UserInfo { user_id: 1, username: "alice", status: Online, signature: Some("Hello") },
        UserInfo { user_id: 2, username: "bob", status: Away, signature: None }
    ] 
}

Binary breakdown:
- Magic: BC BC
- Type: 10 (UserListUpdate)
- Users count: 02 (varint for 2)
- User 1:
  - User ID: 01 (varint)
  - Username len: 05, Username: 61 6C 69 63 65 ("alice")
  - Status: 01 (Online)
  - Signature flag: 01 (Some)
  - Signature len: 05, Signature: 48 65 6C 6C 6F ("Hello")
- User 2:
  - User ID: 02 (varint)
  - Username len: 03, Username: 62 6F 62 ("bob")
  - Status: 02 (Away)
  - Signature flag: 00 (None)

Hex (compact):
BC BC 10 02 01 05 61 6C 69 63 65 01 01 05 48 65 6C 6C 6F 02 03 62 6F 62 02 00
```

### Enum Encoding Examples

#### UserStatus Enum

```rust
enum UserStatus {
    Offline = 0,
    Online = 1,
    Away = 2,
    Busy = 3,
}
```

Encoded as single `u8` byte:
- `0x00` = Offline
- `0x01` = Online
- `0x02` = Away
- `0x03` = Busy

#### RoomType Enum

```rust
enum RoomType {
    Chat = 0,
    Theater = 1,
}
```

Encoded as single `u8` byte:
- `0x00` = Chat
- `0x01` = Theater

### File Transfer Protocol

#### Chunk Bitmap Format

File transfer progress is tracked using a bitmap where each bit represents one chunk:

```
┌─────────────┬──────────────────────────────────────────┐
│ Chunk Count │ Bitmap Bytes                             │
│ varint      │ (ceil(ChunkCount/8) bytes)               │
└─────────────┴──────────────────────────────────────────┘

Example: 16 chunks, chunks 0,2,5,7 received
Bitmap: 10100101 00000000 (2 bytes)
  - Byte 0: 0xA5 (chunks 0-7: ✓✗✓✗✓✗✓✗)
  - Byte 1: 0x00 (chunks 8-15: all missing)
```

#### Large Message Chunking Protocol

When a message exceeds 64KB, it's automatically split into chunks:

```
┌─────────────────────────────────────────────────────────────┐
│ Chunk Header                                                │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│ Message ID  │ Total Size  │ Chunk Index │ Chunk Data       │
│ 16 bytes    │ 4 bytes LE  │ 4 bytes LE  │ (up to 64KB)     │
└─────────────┴─────────────┴─────────────┴──────────────────┘

Receiver reassembly:
1. Buffer chunks by Message ID
2. Track received chunks via bitmap
3. When all chunks received, concatenate in order
4. Decode complete message using bitcode
```

### Performance Characteristics

#### Space Efficiency

- **Varint**: Saves 30-70% space for small integers (common case: user IDs, counts)
- **Bitcode**: More compact than JSON (typically 2-5x smaller)
- **No Field Names**: Unlike JSON, binary format doesn't repeat field names

#### Benchmarks (estimated)

| Message Type | JSON Size | Binary Size | Reduction |
|---|---|---|---|
| TokenAuth | ~35 bytes | ~12 bytes | 65% |
| ChatText (short) | ~120 bytes | ~45 bytes | 62% |
| FileChunk (1KB) | ~1400 bytes | ~1030 bytes | 26% |
| UserListUpdate (10 users) | ~850 bytes | ~320 bytes | 62% |

### Compatibility Notes

1. **Bitcode Versioning**: The message crate includes a protocol version field for future compatibility
2. **Forward Compatibility**: New fields can be added to structs (appended at end) without breaking old clients
3. **Unknown Fields**: Old clients silently skip unknown fields (bitcode design)
4. **Browser Support**: All modern browsers support ArrayBuffer and necessary binary manipulation APIs

## WASM Binary Parsing Implementation

This section specifies how the frontend (WASM/Leptos) handles binary data serialization/deserialization, ensuring seamless communication with the backend.

### Architecture Overview

The `message` crate is a **shared crate** compiled for both native (server) and WASM (client) targets:

```
┌─────────────────────────────────────────────────────────┐
│                    message crate                        │
│  (shared between backend & frontend)                    │
├──────────────────────┬──────────────────────────────────┤
│  Native Target       │  WASM Target                     │
│  (axum server)       │  (leptos frontend)               │
│                      │                                  │
│  - bitcode encode    │  - bitcode encode (same code)    │
│  - bitcode decode    │  - bitcode decode (same code)    │
│  - tokio runtime     │  - wasm-bindgen interop          │
└──────────────────────┴──────────────────────────────────┘
```

**Key Principle**: The same Rust code handles serialization/deserialization on both ends, ensuring protocol consistency without manual JavaScript parsing.

### WASM Compatibility Requirements

1. **Target Triple**: The `message` crate SHALL support compilation for `wasm32-unknown-unknown` target
2. **No std Dependency**: The `message` crate SHALL use `#![no_std]` or conditional compilation to avoid std-only features in WASM
3. **Bitcode WASM Support**: The project SHALL verify `bitcode` crate's WASM compatibility (it supports WASM via `wasm-bindgen`)
4. **JsValue Interop**: The frontend SHALL use `wasm-bindgen` to bridge Rust types with JavaScript APIs (WebSocket, DataChannel, IndexedDB)

### Frontend Data Flow

#### Incoming Message Flow (Receiving)

```
WebSocket (Signaling) / DataChannel (P2P)
         │
         │ onmessage event (ArrayBuffer)
         ▼
    wasm-bindgen Bridge
         │
         │ Pass to WASM (ptr + len)
         ▼
    message crate (bitcode::decode)
         │
         │ Return JsValue
         ▼
    Leptos Signal Update (messages.push)
```

**Requirement**: WHEN a binary message is received via WebSocket or DataChannel THEN the system SHALL pass the ArrayBuffer to WASM for decoding via `wasm-bindgen`, decode using `bitcode`, and update Leptos reactive state.

#### Outgoing Message Flow (Sending)

```
User Action (send message)
         │
         ▼
    Leptos Component (construct Message)
         │
         ▼
    message crate (bitcode::encode)
         │
         │ Returns Vec<u8>
         ▼
    DataChannel.send(ArrayBuffer)
```

**Requirement**: WHEN a user sends a message THEN the system SHALL encode it using `bitcode` in WASM and send via DataChannel as ArrayBuffer.

### WASM Interface Requirements

The `message` crate SHALL expose the following `wasm-bindgen` interfaces:

**Encode Function**:
- Input: Message struct (as JsValue or raw pointer)
- Output: `Vec<u8>` (automatically converted to Uint8Array by wasm-bindgen)
- Purpose: Encode Rust message to binary for sending via DataChannel/WebSocket

**Decode Function**:
- Input: `&[u8]` (binary data from ArrayBuffer)
- Output: Message struct (as JsValue)
- Purpose: Decode binary data to Rust message for processing in Leptos

**Error Handling**:
- All decode errors SHALL be converted to JavaScript-friendly error messages
- Invalid magic number, unknown message type, or corrupted payload SHALL return descriptive error

### Memory Management Requirements

**ArrayBuffer Lifetime**:
- ArrayBuffer is owned by JavaScript garbage collector
- When passed to WASM, it's a borrow (ptr + len), not a copy
- WASM code SHALL NOT retain pointers to ArrayBuffer data beyond the function call

**Memory Leak Prevention**:
- Decode immediately in the event handler, don't store raw binary data longer than necessary
- For file chunks, write to IndexedDB directly without intermediate buffering
- Use `wasm-bindgen`'s automatic memory management for JsValue conversions

**Large File Handling**:
- For large files (>64KB), process in chunks without copying entire file into memory
- Use streaming approach: receive chunk → decode → write to IndexedDB → discard

### Large Message Handling Requirements

**Chunked Message Reassembly**:
- WHEN a message exceeds 64KB THEN the system SHALL split it into chunks (see Binary Protocol Specification)
- The receiver SHALL maintain a reassembly buffer indexed by `message_id`
- Track received chunks using bitmap (see Chunk Bitmap Format)
- WHEN all chunks are received THEN concatenate and decode the complete message
- IF any chunk is missing THEN the system SHALL wait for retransmission (timeout: 30 seconds)

**Reassembly Buffer Cleanup**:
- Complete messages SHALL be removed from reassembly buffer immediately after decoding
- Incomplete messages SHALL be removed after 30-second timeout to prevent memory leaks
- Maximum concurrent reassembly buffers: 10 (to prevent DoS attacks)

### Error Handling Requirements

**Error Types**:
- Invalid magic number (0xBCBC expected)
- Unknown message type discriminator
- Bitcode decode error (corrupted payload)
- Chunk missing (incomplete message)
- Buffer overflow (too many concurrent reassembly buffers)

**Error Reporting**:
- All errors SHALL be converted to JavaScript-friendly format (JsValue with descriptive string)
- Errors SHALL be logged to browser console in debug mode
- Invalid messages SHALL be silently dropped (no user-facing error prompts for malformed data)

### Performance Requirements

**Latency Targets**:
- Message encode/decode latency SHALL NOT exceed 1ms for typical messages (<1KB)
- Message encode/decode latency SHALL NOT exceed 10ms for large messages (<1MB)
- Chunk reassembly overhead SHALL NOT exceed 5ms per chunk

**Optimization Strategies**:
- Use zero-copy deserialization where possible (borrowed strings)
- Enable WASM SIMD optimization when browser supports it (future enhancement)
- Minimize allocations by reusing buffers

### WASM Testing Requirements

**Unit Tests** (via `wasm-bindgen-test`):
- Encode/decode roundtrip for all message types
- Chunk reassembly for large messages (>64KB)
- Error handling for invalid payloads

**Integration Tests**:
- WebSocket binary message receive/decode flow
- DataChannel binary message send/encode flow
- End-to-end: construct → encode → decode → verify

**Test Coverage**:
- Message crate (serialization/deserialization): ≥ 90% line coverage
- Chunk reassembly logic: ≥ 80% line coverage

### Browser API Configuration

**DataChannel Binary Mode**:
- DataChannel SHALL be configured with `binaryType = "arraybuffer"`
- Onmessage handler SHALL check `event.data instanceof ArrayBuffer`
- String messages SHALL be rejected (error logged)

**WebSocket Binary Mode**:
- WebSocket SHALL be configured with `binaryType = "arraybuffer"`
- Onmessage handler SHALL check `event.data instanceof ArrayBuffer`
- JSON messages SHALL be rejected (error logged)

### WASM Binary Parsing Checklist

- [ ] `message` crate compiles for `wasm32-unknown-unknown` target
- [ ] `bitcode` dependency verified to support WASM
- [ ] `wasm-bindgen` encode/decode functions defined
- [ ] DataChannel and WebSocket configured with `binaryType = "arraybuffer"`
- [ ] Large message chunking implemented (messages >64KB)
- [ ] Reassembly buffer cleanup implemented (30s timeout, max 10 concurrent)
- [ ] Error handling converts Rust errors to JavaScript-friendly format
- [ ] WASM tests written with `wasm-bindgen-test`
- [ ] Encode/decode latency meets performance targets (<1ms for <1KB messages)
