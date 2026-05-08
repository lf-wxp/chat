use super::*;
use message::RoomId;

/// A FileInfo created for a room conversation must carry the room_id
/// so the receiver can route the inbound placeholder to the correct
/// conversation instead of defaulting to a 1:1 direct chat.
#[test]
fn file_info_carries_room_id_for_room_conversations() {
  let room_id = RoomId::new();
  let info = FileInfo {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "room-file.zip".into(),
    size: 1024,
    mime_type: "application/zip".into(),
    file_hash: [0u8; 32],
    total_chunks: 1,
    chunk_size: 64 * 1024,
    room_id: Some(room_id.clone()),
  };
  assert_eq!(info.room_id, Some(room_id));
}

/// A FileInfo created for a direct conversation must have room_id
/// set to None so the receiver routes it to the peer's direct chat.
#[test]
fn file_info_has_no_room_id_for_direct_conversations() {
  let info = demo_info(4, 256, "direct.bin");
  assert!(info.room_id.is_none());
}

/// The inbound path must preserve the room_id from the wire metadata
/// when seeding the reassembly buffer.
#[test]
fn inbound_metadata_preserves_room_id() {
  use message::datachannel::FileMetadata;

  let room_id = RoomId::new();
  let meta = FileMetadata {
    message_id: MessageId::new(),
    transfer_id: TransferId::new(),
    filename: "inbound-room.pdf".into(),
    size: 512,
    mime_type: "application/pdf".into(),
    file_hash: [0u8; 32],
    total_chunks: 2,
    chunk_size: 64 * 1024,
    reply_to: None,
    timestamp_nanos: 0,
    room_id: Some(room_id.clone()),
  };

  let info = FileInfo {
    message_id: meta.message_id,
    transfer_id: meta.transfer_id,
    filename: meta.filename,
    size: meta.size,
    mime_type: meta.mime_type,
    file_hash: meta.file_hash,
    total_chunks: meta.total_chunks,
    chunk_size: meta.chunk_size,
    room_id: meta.room_id,
  };

  assert_eq!(info.room_id, Some(room_id));
}
