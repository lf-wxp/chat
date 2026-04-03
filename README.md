# WebRTC Chat Application

A real-time WebRTC-based chat application with end-to-end encryption, supporting text messaging, voice messages with real-time waveform visualization, voice/video calls, file transfers, and shared theater functionality. Built entirely in Rust with a Leptos WASM frontend and an Axum signaling server.

## 🏗️ Architecture

This project is organized as a Cargo workspace (Rust Edition 2024) with three crates:

```
chat/
├── message/     # Shared protocol types & message definitions
├── server/      # Axum-based signaling server
└── client/      # Leptos-based WASM frontend (CSR)
```

### Crate Overview

| Crate | Description | Key Technologies |
|-------|-------------|------------------|
| `message` | Shared message types, protocol definitions, and domain models | `serde`, `bitcode`, `chrono` |
| `server` | WebSocket signaling server with auth, rooms, and content filtering | `axum`, `tokio`, `JWT`, `Argon2`, `aho-corasick` |
| `client` | WebAssembly frontend application (Client-Side Rendering) | `leptos 0.8`, `web-sys`, `IndexedDB`, `Web Audio API` |

## ✨ Features

### Core Features

- **💬 Real-time Messaging**
  - Text, sticker, voice, image, and file messages
  - Message replies with contextual reply bar
  - @mentions with autocomplete dropdown
  - Typing indicators and read receipts
  - Full-text message search with virtual scrolling
  - Drag-and-drop file/image upload

- **🎙️ Voice Messages**
  - Press-and-hold or toggle recording with `MediaRecorder` (WebM/Opus)
  - Real-time waveform visualization during recording via Web Audio `AnalyserNode`
  - Playback with animated waveform bars driven by frequency analysis
  - Duration display and progress indicator
  - Automatic cleanup of audio resources (`AudioContext`, `rAF` loops)

- **📞 Voice/Video Calls**
  - Mesh topology WebRTC (supports up to 8 participants)
  - Adaptive video quality based on network conditions
  - VAD (Voice Activity Detection) for active speaker highlighting
  - Picture-in-Picture floating window
  - Screen sharing support

- **🔒 End-to-End Encryption**
  - ECDH P-256 key exchange
  - HKDF-SHA256 key derivation
  - AES-256-GCM encryption for all messages
  - Forward secrecy — new key pair per session

- **📁 File Transfer**
  - Chunked binary transfer with bitmap progress tracking
  - Automatic flow control and chunk size adjustment
  - Resume support for interrupted transfers
  - 100 MB file size limit

- **🏠 Room System**
  - Create/join rooms with optional password protection
  - Room owner management (kick, mute, transfer ownership)
  - Online user list with real-time presence
  - Invite links for easy sharing

- **🎬 Shared Theater**
  - Synchronized video playback across participants
  - Danmaku (bullet screen) comments
  - Theater controls (play, pause, seek)
  - Multiple video source picker

### Technical Features

- **Binary Protocol**: Full-chain binary transport using `bitcode` serialization
- **Responsive UI**: Mobile-first design with automatic dark theme detection
- **i18n**: Bilingual support (Chinese / English) via `leptos_i18n`
- **Offline Support**: IndexedDB persistence for messages, state, and search index
- **Network Quality Monitoring**: Real-time RTT, packet loss, bandwidth, and jitter tracking with adaptive suggestions
- **Content Filtering**: Server-side sensitive word filtering (Aho-Corasick) and XSS sanitization
- **Virtual Scrolling**: Efficient rendering of large message lists
- **Reusable Components**: Shared UI component library (Avatar, Button, Input, Modal, Toast, VirtualList)

## 🚀 Quick Start

### Prerequisites

- Rust (latest stable) with `wasm32-unknown-unknown` target
- `trunk` for WASM building
- `cargo-make` for task management
- `cargo-watch` for hot reload (optional)

### One-Command Setup

```bash
# Install all required tools (trunk, wasm-pack, cargo-watch, chromedriver, wasm target)
cargo make setup
```

Or install tools manually:

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Install build tools
cargo install trunk cargo-make cargo-watch

# Verify everything is ready
cargo make check-tools
```

### Development

```bash
# Start both signaling server and frontend dev server in parallel (with hot reload)
cargo make dev

# Or run them separately:
cargo make serve-server   # Signaling server on :8888 (cargo-watch hot reload)
cargo make serve-client   # Trunk dev server on :8080 (HMR)
```

### Production Build

```bash
# Full release pipeline: format → lint → test → build
cargo make release

# Or build components individually:
cargo make build-server-release   # Native server binary
cargo make build-client-release   # Optimized WASM bundle
```

### Testing

```bash
# Run all tests (native + WASM)
cargo make test

# Run specific test suites:
cargo make test-native        # Server + message crate tests
cargo make test-client-native # Client pure unit tests
cargo make test-wasm          # WASM tests in headless Chrome
```

### Code Quality

```bash
cargo make fmt    # Format all code
cargo make lint   # Format check + Clippy (native + WASM targets)
```

## 📁 Project Structure

```
chat/
├── Cargo.toml                  # Workspace configuration & shared dependencies
├── Makefile.toml               # cargo-make task runner configuration
├── rustfmt.toml                # Rust formatting rules
├── cspell.json                 # Spell check configuration
│
├── message/                    # Shared protocol crate
│   ├── src/
│   │   ├── lib.rs              # Crate root — re-exports all modules
│   │   ├── chat.rs             # Chat message types (Text, Voice, Image, File, Sticker, System)
│   │   ├── signal.rs           # WebRTC signaling message types (SDP, ICE)
│   │   ├── room.rs             # Room management types (Create, Join, Kick, Mute)
│   │   ├── transfer.rs         # File transfer protocol (Request, Chunk, Ack, Bitmap)
│   │   ├── envelope.rs         # Binary envelope wrapper for framing
│   │   ├── types.rs            # Common shared types
│   │   └── user.rs             # User identity and profile types
│   └── tests/                  # Integration tests
│
├── server/                     # Signaling server crate
│   ├── src/
│   │   ├── main.rs             # Server entry point (Axum + Tokio)
│   │   ├── lib.rs              # Library root
│   │   ├── state.rs            # Application state (DashMap-based concurrent state)
│   │   ├── auth.rs             # JWT authentication & Argon2 password hashing
│   │   ├── room.rs             # Room lifecycle management
│   │   ├── connection.rs       # WebSocket connection handling
│   │   ├── sanitize.rs         # XSS sanitization for user input
│   │   ├── sensitive_filter.rs # Aho-Corasick sensitive word filter
│   │   ├── filter_stats.rs     # Filter statistics and monitoring
│   │   └── handler/            # WebSocket message handlers
│   │       ├── mod.rs          # Handler dispatcher
│   │       ├── auth_handlers.rs
│   │       ├── room_handlers.rs
│   │       ├── signal_router.rs
│   │       ├── invite_handlers.rs
│   │       └── stats_handlers.rs
│   └── tests/                  # Integration tests
│
└── client/                     # WASM frontend crate
    ├── Cargo.toml
    ├── Trunk.toml              # Trunk build configuration
    ├── index.html              # HTML entry point
    ├── build.rs                # Build script (i18n codegen)
    ├── locales/                # i18n translation files (zh, en)
    ├── public/                 # Static assets
    ├── style/                  # CSS stylesheets
    └── src/
        ├── main.rs             # WASM entry point
        ├── app.rs              # Router setup & top-level layout
        ├── crypto.rs           # E2EE implementation (ECDH, HKDF, AES-GCM)
        ├── i18n.rs             # Internationalization setup
        ├── utils.rs            # Shared utility functions
        ├── sticker.rs          # Sticker pack definitions
        ├── pip.rs              # Picture-in-Picture floating window
        ├── vad.rs              # Voice Activity Detection
        ├── flow_control.rs     # File transfer flow control & chunk sizing
        │
        ├── state/              # Global reactive state management
        │   ├── mod.rs          # State module root
        │   ├── provider.rs     # Context provider for all state slices
        │   ├── chat.rs         # Chat state (messages, active chat)
        │   ├── room.rs         # Room state
        │   ├── user.rs         # User identity state
        │   ├── connection.rs   # WebSocket connection state
        │   ├── online_users.rs # Online user presence
        │   ├── search.rs       # Search state
        │   ├── theater.rs      # Theater sync state
        │   ├── theme.rs        # Dark/light theme
        │   ├── ui.rs           # UI state (modals, panels)
        │   ├── vad.rs          # VAD state
        │   └── network_quality.rs
        │
        ├── pages/              # Route-level page components
        │   ├── login.rs        # Login page
        │   ├── settings.rs     # Settings page
        │   ├── chat_view.rs    # Chat view page
        │   ├── room_view.rs    # Room management page
        │   └── home/           # Home page (sidebar + content)
        │       ├── mod.rs
        │       ├── sidebar.rs / sidebar_header.rs
        │       ├── chat_list.rs
        │       ├── room_list.rs
        │       ├── online_user_list.rs
        │       ├── invite_link_panel.rs
        │       └── main_header.rs
        │
        ├── chat/               # Chat UI components
        │   ├── mod.rs          # Chat container (message handling, scroll, encryption)
        │   ├── chat_header.rs  # Chat header with room info
        │   ├── chat_input_bar.rs # Message input with sticker/voice/file buttons
        │   ├── message_list.rs # Virtual-scrolled message list
        │   ├── message_bubble.rs # Individual message bubble renderer
        │   ├── voice_recorder.rs # Voice recording with real-time waveform
        │   ├── voice_recording_bar.rs # Recording UI bar (timer, cancel, send)
        │   ├── voice_bubble.rs # Voice playback bubble with animated waveform
        │   ├── reply_bar.rs    # Reply context bar
        │   ├── mention.rs      # @mention parsing
        │   ├── mention_dropdown.rs # @mention autocomplete
        │   ├── sticker_panel.rs # Sticker picker panel
        │   ├── chat_search.rs / chat_search_bar.rs # In-chat search
        │   ├── drag_overlay.rs / drag_upload.rs # Drag-and-drop upload
        │   └── helpers.rs      # Chat utility functions
        │
        ├── call/               # Voice/Video call components
        │   ├── mod.rs          # Call manager
        │   ├── call_controls.rs # Call control buttons
        │   ├── call_overlay.rs # Call overlay UI
        │   ├── call_video_area.rs # Video display area
        │   ├── video_grid.rs   # Multi-participant video grid
        │   └── types.rs        # Call-related types
        │
        ├── theater/            # Shared theater components
        │   ├── mod.rs          # Theater player & sync logic
        │   ├── danmaku.rs      # Bullet screen comments
        │   └── source_picker.rs # Video source selector
        │
        ├── transfer/           # File transfer components
        │   ├── mod.rs          # Transfer manager & protocol
        │   └── ui.rs           # Transfer progress UI
        │
        ├── components/         # Reusable UI components
        │   ├── avatar.rs       # User avatar
        │   ├── button.rs       # Button component
        │   ├── input.rs        # Input component
        │   ├── modal.rs        # Modal dialog
        │   ├── modal_manager.rs # Global modal manager
        │   ├── toast.rs        # Toast notifications
        │   ├── virtual_list.rs # Virtual scrolling list
        │   ├── misc.rs         # Miscellaneous components
        │   ├── modals/         # Specific modal dialogs
        │   │   ├── incoming_call_modal.rs
        │   │   ├── invite_received_modal.rs
        │   │   └── user_profile_modal.rs
        │   └── network_dashboard/ # Network quality dashboard
        │       ├── mod.rs
        │       ├── metric_card.rs
        │       ├── history_chart.rs
        │       ├── peer_stats.rs
        │       ├── quality_indicator.rs
        │       ├── alerts.rs
        │       └── suggestions.rs
        │
        ├── network_quality/    # Network quality monitoring engine
        │   ├── mod.rs
        │   ├── manager.rs      # Quality metrics collection & analysis
        │   ├── types.rs        # Quality level types & thresholds
        │   └── tests.rs
        │
        ├── services/           # External service integrations
        │   ├── ws/             # WebSocket client
        │   │   ├── mod.rs      # Connection management
        │   │   └── handler.rs  # Message dispatch
        │   └── webrtc/         # WebRTC peer connection
        │       ├── mod.rs      # Peer manager
        │       ├── connection.rs
        │       ├── signaling.rs
        │       ├── datachannel.rs
        │       ├── media.rs
        │       └── crypto.rs   # DTLS/SRTP helpers
        │
        └── storage/            # IndexedDB persistence layer
            ├── mod.rs
            ├── db.rs           # Database schema & CRUD operations
            ├── helpers.rs      # Storage utilities
            └── search.rs       # Full-text search index
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Server bind host | `127.0.0.1` |
| `SERVER_PORT` | Server bind port | `8888` |
| `ICE_SERVERS` | ICE server URLs (JSON array) | `[]` |
| `JWT_SECRET` | JWT signing secret | (required in production) |

### ICE Server Configuration

```bash
# Example: Using Twilio STUN/TURN servers
export ICE_SERVERS='[
  {"urls": "stun:global.stun.twilio.com:3478"},
  {"urls": "turn:global.turn.twilio.com:3478", "username": "...", "credential": "..."}
]'
```

## 🔐 Security

### Authentication Flow

```
Client                          Server
  │                               │
  │──── Connect (WebSocket) ─────>│
  │                               │
  │<─── AuthChallenge (nonce) ────│
  │                               │
  │──── AuthResponse (signed) ───>│
  │                               │
  │<─── AuthResult (JWT token) ───│
  │                               │
  │<─── UserListUpdate ───────────│
  │                               │
  │──── SignalMessage ───────────>│
  │<─── SignalMessage ────────────│
```

### End-to-End Encryption

1. **Key Exchange**: ECDH P-256 on connection establishment
2. **Key Derivation**: HKDF-SHA256 from shared secret
3. **Encryption**: AES-256-GCM for all payloads
4. **Forward Secrecy**: New key pair per session

### Content Filtering

- Aho-Corasick based sensitive word filtering (configurable dictionary)
- XSS sanitization for all user input
- Rate limiting for invitations and messages

## 📊 Network Quality Adaptation

The client continuously monitors network conditions and adjusts:

| Metric | Threshold | Action |
|--------|-----------|--------|
| RTT | > 300ms | Reduce video quality |
| Packet Loss | > 5% | Enable FEC |
| Bandwidth | < 500kbps | Switch to audio-only |
| Jitter | > 50ms | Increase buffer |

## 🌐 Browser Support

| Browser | Version | Notes |
|---------|---------|-------|
| Chrome | 90+ | Full support |
| Firefox | 90+ | Full support |
| Safari | 15+ | Limited WebRTC features |
| Edge | 90+ | Full support |

## 📝 API Reference

### Message Types

All messages are serialized using `bitcode` for efficient binary transport:

```rust
// Chat messages
pub enum ChatMessage {
    Text(TextMessage),
    Sticker(StickerMessage),
    Voice(VoiceMessage),    // WebM/Opus audio with duration
    Image(ImageMessage),
    File(FileMessage),
    System(SystemMessage),
}

// Signaling messages
pub enum SignalMessage {
    SdpOffer(SdpOffer),
    SdpAnswer(SdpAnswer),
    IceCandidate(IceCandidate),
    // ...
}

// File transfer
pub enum TransferMessage {
    Request(TransferRequest),
    Accept(TransferAccept),
    Reject(TransferReject),
    Chunk(TransferChunk),
    Acknowledge(TransferAck),
    // ...
}
```

## 📦 Dependencies

### Server

| Dependency | Purpose |
|------------|---------|
| `axum` | Web framework with WebSocket support |
| `tokio` | Async runtime |
| `jsonwebtoken` | JWT authentication |
| `argon2` | Password hashing |
| `dashmap` | Lock-free concurrent map |
| `aho-corasick` | High-performance string matching for content filtering |
| `tower-http` | CORS and HTTP tracing middleware |

### Client

| Dependency | Purpose |
|------------|---------|
| `leptos` | Reactive UI framework (CSR mode) |
| `leptos_router` | Client-side routing |
| `leptos_i18n` | Internationalization (zh/en) |
| `web-sys` | Web API bindings (WebRTC, Web Audio, IndexedDB, Crypto) |
| `js-sys` | JavaScript interop |
| `gloo-timers` | WASM-compatible timers |
| `wasm-bindgen` | Rust ↔ JavaScript FFI |

### Shared

| Dependency | Purpose |
|------------|---------|
| `bitcode` | Compact binary serialization |
| `serde` | Serialization framework |
| `chrono` | Date/time handling |
| `thiserror` | Ergonomic error types |

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Run the full quality pipeline: `cargo make release`
4. Or run individual checks:
   ```bash
   cargo make fmt     # Format
   cargo make lint    # Clippy + format check
   cargo make test    # All tests
   ```
5. Commit changes: `git commit -am 'Add my feature'`
6. Push to branch: `git push origin feature/my-feature`
7. Submit a pull request

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Leptos](https://leptos.dev/) — Reactive Rust web framework
- [Axum](https://github.com/tokio-rs/axum) — Ergonomic web framework for Rust
- [WebRTC](https://webrtc.org/) — Real-time communication standard
- [Web Audio API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API) — Audio processing and visualization
- [bitcode](https://github.com/SoftbearStudios/bitcode) — Fast binary serialization
