//! WebRTC signaling message handling
//!
//! Handles SDP Offer/Answer and ICE Candidate received from WebSocket signaling server,
//! completing WebRTC connection negotiation.

use leptos::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RtcIceCandidate, RtcIceCandidateInit, RtcSdpType, RtcSessionDescriptionInit};

use message::signal::SignalMessage;

use crate::state;

use super::PeerManager;

/// Handle received SDP Offer (callee)
pub fn handle_sdp_offer(from: &str, _to: &str, sdp: &str) {
  let manager = PeerManager::use_manager();
  let from = from.to_string();
  let sdp = sdp.to_string();

  // Create PeerConnection (if not exists)
  let has_peer = manager.peers.with_value(|peers| peers.contains_key(&from));
  if !has_peer && let Err(e) = manager.create_peer_connection(&from) {
    web_sys::console::error_1(&format!("Failed to create PeerConnection: {e:?}").into());
    return;
  }

  wasm_bindgen_futures::spawn_local(async move {
    let pc = manager
      .peers
      .with_value(|peers| peers.get(&from).map(|e| e.connection.clone()));

    let Some(pc) = pc else { return };

    // Set remote description
    let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    desc.set_sdp(&sdp);
    if let Err(e) = JsFuture::from(pc.set_remote_description(&desc)).await {
      web_sys::console::error_1(&format!("Failed to set remote description: {e:?}").into());
      return;
    }

    // Process cached ICE candidates
    let pending = manager.peers.with_value(|peers| {
      peers
        .get(&from)
        .map(|e| e.pending_ice_candidates.clone())
        .unwrap_or_default()
    });
    for candidate_str in &pending {
      let init = RtcIceCandidateInit::new(candidate_str);
      if let Ok(candidate) = RtcIceCandidate::new(&init) {
        let _ =
          JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate))).await;
      }
    }
    manager.peers.update_value(|peers| {
      if let Some(entry) = peers.get_mut(&from) {
        entry.pending_ice_candidates.clear();
      }
    });

    // Create Answer
    let answer = match JsFuture::from(pc.create_answer()).await {
      Ok(answer) => answer,
      Err(e) => {
        web_sys::console::error_1(&format!("Failed to create Answer: {e:?}").into());
        return;
      }
    };

    let answer_sdp = js_sys::Reflect::get(&answer, &"sdp".into())
      .unwrap()
      .as_string()
      .unwrap_or_default();

    // Set local description
    let local_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    local_desc.set_sdp(&answer_sdp);
    if let Err(e) = JsFuture::from(pc.set_local_description(&local_desc)).await {
      web_sys::console::error_1(&format!("Failed to set local description: {e:?}").into());
      return;
    }

    // Send Answer
    let user_state = state::use_user_state();
    let my_id = user_state.get_untracked().user_id.clone();
    let ws = crate::services::ws::WsClient::use_client();
    let _ = ws.send(&SignalMessage::SdpAnswer {
      from: my_id,
      to: from,
      sdp: answer_sdp,
    });
  });
}

/// Handle received SDP Answer (caller)
pub fn handle_sdp_answer(from: &str, _to: &str, sdp: &str) {
  let manager = PeerManager::use_manager();
  let from = from.to_string();
  let sdp = sdp.to_string();

  wasm_bindgen_futures::spawn_local(async move {
    let pc = manager
      .peers
      .with_value(|peers| peers.get(&from).map(|e| e.connection.clone()));

    let Some(pc) = pc else { return };

    let desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    desc.set_sdp(&sdp);
    if let Err(e) = JsFuture::from(pc.set_remote_description(&desc)).await {
      web_sys::console::error_1(&format!("Failed to set remote description: {e:?}").into());
      return;
    }

    // Process cached ICE candidates
    let pending = manager.peers.with_value(|peers| {
      peers
        .get(&from)
        .map(|e| e.pending_ice_candidates.clone())
        .unwrap_or_default()
    });
    for candidate_str in &pending {
      let init = RtcIceCandidateInit::new(candidate_str);
      if let Ok(candidate) = RtcIceCandidate::new(&init) {
        let _ =
          JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate))).await;
      }
    }
    manager.peers.update_value(|peers| {
      if let Some(entry) = peers.get_mut(&from) {
        entry.pending_ice_candidates.clear();
      }
    });
  });
}

/// Handle received ICE Candidate
pub fn handle_ice_candidate(from: &str, _to: &str, candidate: &str) {
  let manager = PeerManager::use_manager();
  let from = from.to_string();
  let candidate = candidate.to_string();

  // Check if remote description is set
  let has_remote_desc = manager.peers.with_value(|peers| {
    peers
      .get(&from)
      .is_some_and(|e| e.connection.remote_description().is_some())
  });

  if has_remote_desc {
    // Add directly
    wasm_bindgen_futures::spawn_local(async move {
      let pc = manager
        .peers
        .with_value(|peers| peers.get(&from).map(|e| e.connection.clone()));

      if let Some(pc) = pc {
        let init = RtcIceCandidateInit::new(&candidate);
        if let Ok(ice_candidate) = RtcIceCandidate::new(&init) {
          let _ =
            JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_candidate)))
              .await;
        }
      }
    });
  } else {
    // Cache and add after remote description is set
    manager.peers.update_value(|peers| {
      if let Some(entry) = peers.get_mut(&from) {
        entry.pending_ice_candidates.push(candidate);
      }
    });
  }
}
