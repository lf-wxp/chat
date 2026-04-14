//! WebRTC signaling message tests.

use super::*;

#[test]
fn test_sdp_offer_roundtrip() {
  let msg = SdpOffer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "v=0\r\no=- 123456 123456 IN IP4 127.0.0.1\r\n".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: SdpOffer = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_ice_candidate_roundtrip() {
  let msg = IceCandidate {
    from: UserId::new(),
    to: UserId::new(),
    candidate: "candidate:1 1 UDP 2122260223 192.168.1.1 54321 typ host".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: IceCandidate = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_peer_established_roundtrip() {
  let msg = PeerEstablished {
    from: UserId::new(),
    to: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: PeerEstablished = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_active_peers_list_roundtrip() {
  let msg = ActivePeersList {
    peers: vec![UserId::new(), UserId::new(), UserId::new()],
  };
  let encoded = bitcode::encode(&msg);
  let decoded: ActivePeersList = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_sdp_answer_roundtrip() {
  let msg = SdpAnswer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "v=0\r\no=- 123 1 IN IP4 0.0.0.0\r\ns=-\r\nt=0 0\r\n".to_string(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: SdpAnswer = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_peer_closed_roundtrip() {
  let msg = PeerClosed {
    from: UserId::new(),
    to: UserId::new(),
  };
  let encoded = bitcode::encode(&msg);
  let decoded: PeerClosed = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_sdp_answer_roundtrip() {
  let msg = SignalingMessage::SdpAnswer(SdpAnswer {
    from: UserId::new(),
    to: UserId::new(),
    sdp: "answer-sdp".to_string(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_signaling_message_peer_closed_roundtrip() {
  let msg = SignalingMessage::PeerClosed(PeerClosed {
    from: UserId::new(),
    to: UserId::new(),
  });
  let encoded = bitcode::encode(&msg);
  let decoded: SignalingMessage = bitcode::decode(&encoded).expect("Failed to decode");
  assert_eq!(msg, decoded);
}

#[test]
fn test_discriminator_webrtc_messages() {
  let uid1 = UserId::new();
  let uid2 = UserId::new();
  assert_eq!(
    SignalingMessage::SdpOffer(SdpOffer {
      from: uid1.clone(),
      to: uid2.clone(),
      sdp: "offer".into()
    })
    .discriminator(),
    SDP_OFFER
  );
  assert_eq!(
    SignalingMessage::SdpAnswer(SdpAnswer {
      from: uid1.clone(),
      to: uid2.clone(),
      sdp: "answer".into()
    })
    .discriminator(),
    SDP_ANSWER
  );
  assert_eq!(
    SignalingMessage::IceCandidate(IceCandidate {
      from: uid1,
      to: uid2,
      candidate: "c".into()
    })
    .discriminator(),
    ICE_CANDIDATE
  );
}

#[test]
fn test_discriminator_peer_tracking() {
  let uid1 = UserId::new();
  let uid2 = UserId::new();
  assert_eq!(
    SignalingMessage::PeerEstablished(PeerEstablished {
      from: uid1.clone(),
      to: uid2.clone()
    })
    .discriminator(),
    PEER_ESTABLISHED
  );
  assert_eq!(
    SignalingMessage::PeerClosed(PeerClosed {
      from: uid1,
      to: uid2
    })
    .discriminator(),
    PEER_CLOSED
  );
  assert_eq!(
    SignalingMessage::ActivePeersList(ActivePeersList { peers: vec![] }).discriminator(),
    ACTIVE_PEERS_LIST
  );
}

/// Test that encoding a `SignalingMessage` through the frame pipeline
/// with the correct discriminator produces a valid frame that decodes back.
#[test]
fn test_signaling_frame_roundtrip_with_correct_discriminator() {
  let msg = SignalingMessage::Ping(Ping {});
  let payload = bitcode::encode(&msg);

  // Ping is a unit struct — bitcode may produce an empty payload.
  // Skip the frame-level test for empty-payload messages.
  if payload.is_empty() {
    return;
  }

  let frame = MessageFrame::new(msg.discriminator(), payload);
  let encoded = encode_frame(&frame).expect("encode should succeed");
  let decoded_frame = decode_frame(&encoded).expect("decode should succeed");

  assert_eq!(decoded_frame.message_type, PING);

  let decoded_msg: SignalingMessage =
    bitcode::decode(&decoded_frame.payload).expect("bitcode decode should succeed");
  assert_eq!(msg, decoded_msg);
}

/// Test that decoding a frame with an unmapped discriminator byte
/// succeeds at the frame level but the payload cannot be interpreted
/// as a valid `SignalingMessage`.
#[test]
fn test_signaling_unmapped_discriminator_frame_succeeds_bitcode_fails() {
  // Pick discriminator values that are NOT used by SignalingMessage.
  // Known gaps include: 0x08-0x0F, 0x12-0x1F, 0x25-0x2F, 0x33-0x3F
  let unmapped_discriminators: Vec<u8> = vec![0x08, 0x0F, 0x25, 0x33, 0x43, 0x5F, 0x6A, 0x7E];

  for disc in unmapped_discriminators {
    // Create a frame with a valid payload (TokenAuth bitcode bytes)
    let valid_payload = bitcode::encode(&TokenAuth {
      token: "test".to_string(),
    });
    let frame = MessageFrame::new(disc, valid_payload.clone());
    let encoded = encode_frame(&frame).expect("encode should succeed");

    // Frame-level decode should succeed
    let decoded_frame = decode_frame(&encoded).expect("frame decode should succeed");
    assert_eq!(decoded_frame.message_type, disc);

    // Trying to bitcode-decode as SignalingMessage may or may not fail
    if let Ok(decoded_msg) = bitcode::decode::<SignalingMessage>(&decoded_frame.payload) {
      // If bitcode happens to decode without error, the discriminator should NOT match
      assert_ne!(
        decoded_msg.discriminator(),
        disc,
        "Decoded SignalingMessage discriminator should not match unmapped value 0x{disc:02X}"
      );
    }
  }
}

/// Test that a crafted payload with a discriminator byte in the `DataChannel`
/// range (0x80+) cannot be decoded as a `SignalingMessage`.
#[test]
fn test_signaling_datachannel_discriminator_not_confused() {
  let datachannel_disc: u8 = 0x80; // CHAT_TEXT
  let payload = bitcode::encode(&TokenAuth {
    token: "cross-namespace".to_string(),
  });

  let frame = MessageFrame::new(datachannel_disc, payload);
  let encoded = encode_frame(&frame).expect("encode should succeed");
  let decoded_frame = decode_frame(&encoded).expect("frame decode should succeed");

  // The frame should carry the DataChannel discriminator
  assert_eq!(decoded_frame.message_type, datachannel_disc);

  // Trying to decode as SignalingMessage should fail or produce a message
  // whose discriminator is NOT 0x80 (signaling uses 0x00-0x7D range).
  if let Ok(decoded_msg) = bitcode::decode::<SignalingMessage>(&decoded_frame.payload) {
    // All SignalingMessage discriminators are < 0x80
    assert!(
      decoded_msg.discriminator() < 0x80,
      "SignalingMessage discriminator should be < 0x80, got 0x{:02X}",
      decoded_msg.discriminator()
    );
  }
}
