//! E2EE encrypted message sending

use message::envelope::Envelope;

use super::PeerManager;

impl PeerManager {
  /// Send message through E2EE encryption (if shared key is established, otherwise send in plaintext)
  ///
  /// Serialize raw Payload and encrypt with AES-256-GCM,
  /// wrap as `Payload::Encrypted` to send.
  pub fn send_envelope_encrypted(&self, remote_user_id: &str, envelope: &Envelope) {
    if !crate::crypto::has_shared_key(remote_user_id) {
      // Shared key not established yet, send in plaintext
      let _ = self.send_envelope(remote_user_id, envelope);
      return;
    }

    let remote_id = remote_user_id.to_string();
    let from = envelope.from.clone();
    let to = envelope.to.clone();
    let payload = envelope.payload.clone();
    let self_clone = self.clone();

    wasm_bindgen_futures::spawn_local(async move {
      // Serialize raw Payload
      let plaintext = match bitcode::serialize(&payload) {
        Ok(bytes) => bytes,
        Err(e) => {
          web_sys::console::error_1(&format!("E2EE: Payload serialization failed: {e}").into());
          return;
        }
      };

      // AES-256-GCM encryption
      match crate::crypto::encrypt(&remote_id, &plaintext).await {
        Ok((iv, ciphertext)) => {
          let encrypted_envelope = Envelope::new(
            from,
            to,
            message::envelope::Payload::Encrypted(message::envelope::EncryptedPayload {
              iv,
              ciphertext,
            }),
          );
          if let Err(e) = self_clone.send_envelope(&remote_id, &encrypted_envelope) {
            web_sys::console::error_1(&format!("E2EE: Encrypted message send failed: {e}").into());
          }
        }
        Err(e) => {
          web_sys::console::error_1(&format!("E2EE: Encryption failed: {e}").into());
          // Fallback to plaintext when encryption fails
          let fallback_envelope = Envelope::new(from, to, payload);
          let _ = self_clone.send_envelope(&remote_id, &fallback_envelope);
        }
      }
    });
  }

  /// Broadcast encrypted message to all connected peers
  ///
  /// Encrypt separately for each peer (because each pair has a different shared key).
  pub fn broadcast_envelope_encrypted(&self, envelope: &Envelope) {
    let peers = self.connected_peers();
    for peer_id in &peers {
      self.send_envelope_encrypted(peer_id, envelope);
    }
  }
}
