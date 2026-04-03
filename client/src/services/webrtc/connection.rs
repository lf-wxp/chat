//! WebRTC connection creation and SDP negotiation

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  RtcConfiguration, RtcDataChannelEvent, RtcDataChannelInit, RtcPeerConnection,
  RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent,
};

use leptos::prelude::*;
use message::signal::SignalMessage;

use crate::state;

use super::datachannel::setup_data_channel_handlers;
use super::{DATA_CHANNEL_LABEL, PeerEntry, PeerManager};

impl PeerManager {
  /// Create RTCPeerConnection configuration
  pub(super) fn create_rtc_config() -> RtcConfiguration {
    let config = RtcConfiguration::new();
    let ice_servers = js_sys::Array::new();

    for server_url in Self::ice_servers() {
      let ice_server = js_sys::Object::new();
      let urls = js_sys::Array::new();
      urls.push(&JsValue::from_str(&server_url));
      js_sys::Reflect::set(&ice_server, &"urls".into(), &urls).unwrap();
      ice_servers.push(&ice_server);
    }

    config.set_ice_servers(&ice_servers);
    config
  }

  /// Create PeerConnection for specified remote user
  pub(super) fn create_peer_connection(&self, remote_user_id: &str) -> Result<(), JsValue> {
    let config = Self::create_rtc_config();
    let pc = RtcPeerConnection::new_with_configuration(&config)?;

    let remote_id = remote_user_id.to_string();

    // ---- onicecandidate ----
    let remote_id_ice = remote_id.clone();
    let onicecandidate =
      Closure::<dyn Fn(RtcPeerConnectionIceEvent)>::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
          let candidate_str = candidate.candidate();
          if !candidate_str.is_empty() {
            let user_state = state::use_user_state();
            let from = user_state.get_untracked().user_id.clone();
            let ws = crate::services::ws::WsClient::use_client();
            let _ = ws.send(&SignalMessage::IceCandidate {
              from,
              to: remote_id_ice.clone(),
              candidate: candidate_str,
            });
          }
        }
      });
    pc.set_onicecandidate(Some(onicecandidate.as_ref().unchecked_ref()));
    onicecandidate.forget();

    // ---- ondatachannel (passive side receives DataChannel) ----
    let self_clone = self.clone();
    let remote_id_dc = remote_id.clone();
    let ondatachannel =
      Closure::<dyn Fn(RtcDataChannelEvent)>::new(move |ev: RtcDataChannelEvent| {
        let dc = ev.channel();
        web_sys::console::log_1(&format!("Received DataChannel: label={}", dc.label()).into());
        setup_data_channel_handlers(&dc, &remote_id_dc);
        self_clone.peers.update_value(|peers| {
          if let Some(entry) = peers.get_mut(&remote_id_dc) {
            entry.data_channel = Some(dc);
          }
        });
      });
    pc.set_ondatachannel(Some(ondatachannel.as_ref().unchecked_ref()));
    ondatachannel.forget();

    // ---- ontrack (receive remote media tracks) ----
    let remote_id_track = remote_id.clone();
    let ontrack = Closure::<dyn Fn(RtcTrackEvent)>::new(move |ev: RtcTrackEvent| {
      web_sys::console::log_1(
        &format!("Received remote media track: user={remote_id_track}").into(),
      );
      let streams = ev.streams();
      if streams.length() > 0
        && let Some(stream) = streams.get(0).dyn_ref::<web_sys::MediaStream>()
      {
        let video_id = format!("remote-video-{remote_id_track}");
        let video_el = web_sys::window().and_then(|w| w.document()).and_then(|d| {
          d.get_element_by_id(&video_id)
            .or_else(|| d.get_element_by_id("remote-video"))
        });

        if let Some(video) = video_el {
          let video: web_sys::HtmlVideoElement = video.unchecked_into();
          video.set_src_object(Some(stream));
          let _ = video.play();
        }

        // Start VAD speaker detection
        if let Some(vad_mgr) = leptos::prelude::use_context::<crate::vad::VadManager>() {
          vad_mgr.add_stream(&remote_id_track, stream);
        }
      }
    });
    pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
    ontrack.forget();

    let entry = PeerEntry {
      connection: pc,
      data_channel: None,
      remote_user_id: remote_id.clone(),
      pending_ice_candidates: Vec::new(),
    };

    self.peers.update_value(|peers| {
      peers.insert(remote_id, entry);
    });

    Ok(())
  }

  /// Initiate SDP Offer (caller)
  pub fn create_offer(&self, remote_user_id: &str) {
    let remote_id = remote_user_id.to_string();

    // Create PeerConnection if not exists
    let has_peer = self
      .peers
      .with_value(|peers| peers.contains_key(&remote_id));
    if !has_peer && let Err(e) = self.create_peer_connection(&remote_id) {
      web_sys::console::error_1(&format!("Failed to create PeerConnection: {e:?}").into());
      return;
    }

    // Create DataChannel (initiator creates it)
    let dc_init = RtcDataChannelInit::new();

    self.peers.update_value(|peers| {
      if let Some(entry) = peers.get_mut(&remote_id) {
        let dc = entry
          .connection
          .create_data_channel_with_data_channel_dict(DATA_CHANNEL_LABEL, &dc_init);
        setup_data_channel_handlers(&dc, &remote_id);
        entry.data_channel = Some(dc);
      }
    });

    // Create Offer
    let self_clone = self.clone();
    let remote_id2 = remote_id.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let pc = self_clone
        .peers
        .with_value(|peers| peers.get(&remote_id2).map(|e| e.connection.clone()));

      let Some(pc) = pc else { return };

      let offer = match JsFuture::from(pc.create_offer()).await {
        Ok(offer) => offer,
        Err(e) => {
          web_sys::console::error_1(&format!("Failed to create Offer: {e:?}").into());
          return;
        }
      };

      let offer_sdp = js_sys::Reflect::get(&offer, &"sdp".into())
        .unwrap()
        .as_string()
        .unwrap_or_default();

      let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
      desc.set_sdp(&offer_sdp);
      if let Err(e) = JsFuture::from(pc.set_local_description(&desc)).await {
        web_sys::console::error_1(&format!("Failed to set local description: {e:?}").into());
        return;
      }

      let user_state = state::use_user_state();
      let from = user_state.get_untracked().user_id.clone();
      let ws = crate::services::ws::WsClient::use_client();
      let _ = ws.send(&SignalMessage::SdpOffer {
        from,
        to: remote_id2,
        sdp: offer_sdp,
      });
    });
  }
}
