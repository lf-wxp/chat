# Requirement 11: Message Persistence & Offline Support

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want chat history not to be lost when I refresh the page, so that I can review historical messages at any time.

> **Constraint:** Since the server does not do persistent storage, message persistence relies entirely on client-side local storage (IndexedDB). The server only temporarily stores a small number of offline invitations (not chat messages) in memory; chat messages are transported via DataChannel P2P, the server does not handle them.

## Acceptance Criteria

1. WHEN a user sends or receives a message THEN the system SHALL store the message in **decrypted plaintext form** in the browser's IndexedDB (E2EE protects transport link security, local storage security relies on browser sandbox isolation; storing plaintext avoids the problem of old messages being undecryptable after key refresh)
2. WHEN a user reopens the app THEN the system SHALL load historical chat records from IndexedDB
3. WHEN a user is offline and other users send P2P messages THEN since the DataChannel connection is broken, these messages cannot be delivered; the system SHALL implement the following message acknowledgment and resend mechanism:
   - **Message ACK Mechanism**: Each message carries a unique `message_id` (UUID), the receiver SHALL send an ACK confirmation via DataChannel upon receiving a message (`MessageAck { message_id }`); the sender SHALL maintain a **per-peer granularity** "unacknowledged message queue" locally (stored in IndexedDB, indexed by `(message_id, peer_id)`), removing from that Peer's queue upon receiving that Peer's ACK; in multi-user chat scenarios, the UI SHALL display aggregated delivery status for messages (e.g., "Delivered to 3/5 people"), showing "Delivered" when all Peers have acknowledged
   - **Auto Resend**: WHEN a DataChannel is re-established THEN the sender SHALL automatically check the unacknowledged message queue for that Peer, and automatically resend all unacknowledged messages (no manual user action needed)
   - **Deduplication**: The receiver SHALL deduplicate repeated messages based on `message_id`, avoiding duplicate display
   - **Expiry Cleanup**: Unacknowledged messages are retained in the queue for a maximum of 72 hours (default, configurable via Settings panel with options: 24h / 72h / 7 days), after which they are automatically cleaned up and the message status is marked as "Send failed"
4. IF IndexedDB storage space is insufficient THEN the system SHALL automatically clean up the earliest message records, and prompt the user
5. WHEN a user receives a connection invitation while offline THEN the signaling server SHALL temporarily store the invitation in memory, pushing it when the user comes online (invitation timeout rules still apply; invitations stored in memory are lost on server restart)
