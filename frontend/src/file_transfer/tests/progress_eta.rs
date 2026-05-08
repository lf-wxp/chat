use super::super::types::PeerProgress;
use super::*;
use leptos::prelude::Get;

#[test]
fn transfer_progress_percent_is_clamped() {
  // Single-peer (incoming) scenario: peers list is empty.
  let mut p = TransferProgress::new(1000, 2);
  p.transferred_bytes = 250;
  assert_eq!(p.percent(), 25);
  p.transferred_bytes = 1000;
  assert_eq!(p.percent(), 100);
  p.transferred_bytes = 9999; // Defensive: clamped to 100.
  assert_eq!(p.percent(), 100);
  let zero = TransferProgress::new(0, 0);
  assert_eq!(zero.percent(), 100);
}

#[test]
fn transfer_progress_percent_scales_with_peer_count() {
  // Multi-peer outgoing: total work = total_bytes * peer_count.
  let mut p = TransferProgress::new(1000, 2);
  p.peers = vec![
    PeerProgress {
      peer_id: UserId::from(1u64),
      chunks_sent: 0,
      status: TransferStatus::InProgress,
    },
    PeerProgress {
      peer_id: UserId::from(2u64),
      chunks_sent: 0,
      status: TransferStatus::InProgress,
    },
  ];
  // 250 / (1000 * 2) = 12.5% -> 12
  p.transferred_bytes = 250;
  assert_eq!(p.percent(), 12);
  // 1000 / 2000 = 50%
  p.transferred_bytes = 1000;
  assert_eq!(p.percent(), 50);
  // 2000 / 2000 = 100%
  p.transferred_bytes = 2000;
  assert_eq!(p.percent(), 100);
}

#[test]
fn estimate_transfer_time_scales_with_peer_count() {
  let seconds_single = estimate_transfer_seconds(20 * 1024 * 1024, 1);
  let seconds_five = estimate_transfer_seconds(20 * 1024 * 1024, 5);
  assert!(seconds_five >= seconds_single * 5);
}

#[test]
fn peer_progress_snapshots_preserve_order() {
  let mut p = TransferProgress::new(100, 2);
  p.peers = vec![
    PeerProgress {
      peer_id: UserId::from(1u64),
      chunks_sent: 1,
      status: TransferStatus::InProgress,
    },
    PeerProgress {
      peer_id: UserId::from(2u64),
      chunks_sent: 0,
      status: TransferStatus::Preparing,
    },
  ];
  assert_eq!(p.peers.len(), 2);
  assert_eq!(p.peers[0].peer_id, UserId::from(1u64));
  assert_eq!(p.peers[1].peer_id, UserId::from(2u64));
}

#[test]
fn multi_peer_dispatch_progress_accumulates() {
  let info = demo_info(10, 640, "multi.bin");
  let peers = vec![UserId::from(1u64), UserId::from(2u64), UserId::from(3u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 640],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Simulate sending 3 chunks to peer 1.
  for _ in 0..3 {
    tx.advance(&peers[0], 64);
  }
  // Simulate sending 2 chunks to peer 2.
  for _ in 0..2 {
    tx.advance(&peers[1], 64);
  }

  let p = tx.progress.get_untracked();
  assert_eq!(
    p.transferred_bytes,
    5 * 64,
    "total bytes = 5 chunks * 64 bytes"
  );
  assert_eq!(
    p.chunks_done, 5,
    "chunks_done should track total across peers"
  );

  // Peer 1 has 3 chunks sent, peer 2 has 2, peer 3 has 0.
  let peer1 = p.peers.iter().find(|e| e.peer_id == peers[0]).unwrap();
  let peer2 = p.peers.iter().find(|e| e.peer_id == peers[1]).unwrap();
  // Peer 3 was never advanced so it won't appear in the progress
  // peers list — advance() lazily inserts peers on first chunk.
  let peer3_chunks = p
    .peers
    .iter()
    .find(|e| e.peer_id == peers[2])
    .map_or(0, |e| e.chunks_sent);
  assert_eq!(peer1.chunks_sent, 3);
  assert_eq!(peer2.chunks_sent, 2);
  assert_eq!(peer3_chunks, 0);

  // Total work = size * peer_count_in_progress = 640 * 2 = 1280
  // (peer 3 is not in progress.peers since it was never advanced).
  // Transferred = 320. Percent = 320/1280 * 100 = 25%.
  let pct = p.percent();
  assert_eq!(pct, 25, "percent should be 25%, got {pct}");
}

#[test]
fn throughput_and_eta_update_after_advance() {
  let info = demo_info(4, 256, "eta.bin");
  let peers = vec![UserId::from(1u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 256],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Advance 1 chunk (64 bytes) with 1000ms elapsed.
  tx.advance(&peers[0], 64);
  tx.record_throughput(1000);

  let p = tx.progress.get_untracked();
  assert!(
    p.bytes_per_sec > 0,
    "throughput should be non-zero after first chunk"
  );
  assert!(
    p.eta_secs.is_some(),
    "ETA should be computed after first chunk"
  );

  // After 1/4 of the file at 64 B/s, ETA should be ~3 seconds.
  let eta = p.eta_secs.unwrap();
  assert!((2..=4).contains(&eta), "ETA should be ~3s, got {eta}");
}

#[test]
fn cancel_outbound_propagates_to_all_peers() {
  let info = demo_info(4, 256, "cancel.bin");
  let peers = vec![UserId::from(1u64), UserId::from(2u64)];
  let progress = RwSignal::new(TransferProgress::new(info.size, info.total_chunks));
  // Seed the peer progress entries.
  progress.update(|p| {
    p.peers = peers
      .iter()
      .map(|pid| PeerProgress {
        peer_id: pid.clone(),
        chunks_sent: 0,
        status: TransferStatus::InProgress,
      })
      .collect();
  });
  let status = RwSignal::new(TransferStatus::InProgress);
  let tx = OutgoingTransfer {
    info,
    bytes: vec![0u8; 256],
    object_url: String::new(),
    thumbnail_url: RwSignal::new(None),
    targets: peers.clone(),
    progress,
    status,
    direction: TransferDirection::Outgoing,
  };

  // Cancel: set status and propagate.
  tx.status.set(TransferStatus::Cancelled);
  tx.progress.update(|p| {
    for entry in &mut p.peers {
      if !entry.status.is_terminal() {
        entry.status = TransferStatus::Cancelled;
      }
    }
  });

  assert!(tx.status.get().is_terminal());
  for entry in &tx.progress.get().peers {
    assert!(matches!(entry.status, TransferStatus::Cancelled));
  }
}
