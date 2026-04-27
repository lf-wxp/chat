//! A single video tile in the call grid.
//!
//! Renders one participant: the `<video>` surface, a name label, an
//! optional "speaking" pulse, and — for remote peers — a small network
//! quality indicator in the footer. When no media stream is attached
//! (camera off, audio-only call, peer not yet publishing) we render an
//! Identicon-based avatar placeholder over the video element so the
//! tile always shows *something* (Req 3.6 / 10.6.28 — P2-7 fix).
//!
//! Remote-peer tiles additionally render three status icons driven by
//! the peer's `MediaStateUpdate` / `ReconnectingState` broadcasts
//! (Req 3.5 / 7.1 / 10.5.24): muted microphone, camera disabled,
//! reconnecting.

use leptos::html;
use leptos::prelude::*;
use message::UserId;
use web_sys::{HtmlVideoElement, MediaStream};

use crate::components::call::NetworkIndicator;
use crate::identicon::generate_identicon_data_uri;

/// Compute the BEM class list for a video tile given its flags.
///
/// Returned as a `Vec<&'static str>` joined by the caller (via
/// `parts.join(" ")`) so the helper stays allocation-light for tests.
/// Exposed as a pure function so the rendering logic is unit-testable
/// without mounting the component (round-4 coverage fix).
#[must_use]
pub fn tile_class_parts(
  hero: bool,
  speaking: bool,
  is_local: bool,
  stream_present: bool,
  reconnecting: bool,
) -> Vec<&'static str> {
  let mut parts = vec!["video-tile"];
  if hero {
    parts.push("video-tile--hero");
  }
  if speaking {
    parts.push("video-tile--speaking");
  }
  if is_local {
    parts.push("video-tile--local");
  }
  if !stream_present {
    parts.push("video-tile--no-stream");
  }
  if reconnecting && !is_local {
    parts.push("video-tile--reconnecting");
  }
  parts
}

/// Classify a tile as a `data-pip-candidate` role ("hero" / "local" /
/// "peer"), used by the control bar to pick a `<video>` element for
/// Picture-in-Picture (Req 7.3 — P2-New-2 fix).
#[must_use]
pub const fn pip_candidate_role(hero: bool, is_local: bool) -> &'static str {
  if hero {
    "hero"
  } else if is_local {
    "local"
  } else {
    "peer"
  }
}

/// Whether the "muted mic / camera off / reconnecting" icons should be
/// rendered on a tile. These broadcasts only make sense for remote
/// peers; the local tile exposes the same info through the control bar.
#[must_use]
pub const fn should_show_remote_media_icons(is_local: bool) -> bool {
  !is_local
}

/// Render a single participant tile.
#[component]
pub fn VideoTile(
  /// User id displayed in the tile label and used to look up network
  /// quality in `AppState::network_quality`.
  user_id: UserId,
  /// Display name for the tile header.
  display_name: String,
  /// Optional stream to render in the `<video>` element.
  stream: Option<MediaStream>,
  /// Whether VAD currently flags this participant as speaking.
  speaking: bool,
  /// Whether the stream is the local preview (mirrored + muted).
  is_local: bool,
  /// Render the tile at the dominant "hero" size (screen-share).
  #[prop(default = false)]
  hero: bool,
  /// Whether the participant's microphone is enabled. Defaults to true
  /// (no icon rendered) for backwards compatibility with callers who
  /// did not pass the flag. Only meaningful for remote tiles.
  #[prop(default = true)]
  mic_enabled: bool,
  /// Whether the participant's camera is enabled. Defaults to true.
  /// Only meaningful for remote tiles.
  #[prop(default = true)]
  camera_enabled: bool,
  /// Whether the participant is currently reconnecting. Defaults to
  /// false. Only meaningful for remote tiles.
  #[prop(default = false)]
  reconnecting: bool,
) -> impl IntoView {
  let video_ref: NodeRef<html::Video> = NodeRef::new();
  let stream_for_effect = stream.clone();
  let stream_present = stream.is_some();

  // Attach the stream to the `<video>` element once the element is
  // mounted. Ignoring the `Result` is safe here — the only failure
  // mode is a JS TypeError on `srcObject` assignment which cannot
  // occur for a freshly created element in a standards-compliant
  // browser.
  Effect::new(move |_| {
    if let Some(el) = video_ref.get() {
      let html_video: &HtmlVideoElement = el.as_ref();
      let _ = crate::call::attach_stream_to_video(html_video, stream_for_effect.as_ref());
    }
  });

  let indicator_user = user_id.clone();
  let label = display_name.clone();
  let avatar_label = display_name.clone();
  // Identicon for the placeholder. The avatar key is derived from the
  // display name (matches the registration-time identicon) so the
  // placeholder visually matches the user's normal avatar in the chat
  // sidebar.
  let avatar_uri = generate_identicon_data_uri(&display_name);

  let tile_class =
    move || tile_class_parts(hero, speaking, is_local, stream_present, reconnecting).join(" ");

  // P2-New-2 fix: expose data attributes so `CallControls` can pick
  // the right `<video>` element for Picture-in-Picture without
  // reading CSS class names.
  let pip_candidate = pip_candidate_role(hero, is_local);

  // Only show media-state icons on remote tiles; the local control
  // bar already surfaces this info via the mute / camera buttons.
  let show_remote_media_icons = should_show_remote_media_icons(is_local);

  view! {
    <div class=tile_class role="group" aria-label=label.clone() data-pip-candidate=pip_candidate>
      <video
        node_ref=video_ref
        class="video-tile__video"
        autoplay=true
        playsinline=true
        muted=is_local
      ></video>
      <Show when=move || !stream_present>
        <div class="video-tile__placeholder" aria-hidden="true">
          <img
            class="video-tile__avatar"
            src=avatar_uri.clone()
            alt=avatar_label.clone()
          />
        </div>
      </Show>
      <Show when=move || show_remote_media_icons && reconnecting>
        <div class="video-tile__status video-tile__status--reconnecting" role="status">
          "⟳"
        </div>
      </Show>
      <footer class="video-tile__meta">
        <span class="video-tile__name">{display_name}</span>
        <Show when=move || show_remote_media_icons && !mic_enabled>
          <span class="video-tile__icon video-tile__icon--muted" aria-label="muted">
            "🔇"
          </span>
        </Show>
        <Show when=move || show_remote_media_icons && !camera_enabled>
          <span class="video-tile__icon video-tile__icon--camera-off" aria-label="camera off">
            "📷"
          </span>
        </Show>
        <Show when=move || !is_local>
          <NetworkIndicator peer_id=indicator_user.clone() />
        </Show>
      </footer>
    </div>
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_tile_has_only_base_class() {
    let parts = tile_class_parts(false, false, false, true, false);
    assert_eq!(parts, vec!["video-tile"]);
  }

  #[test]
  fn hero_tile_adds_hero_modifier() {
    let parts = tile_class_parts(true, false, false, true, false);
    assert!(parts.contains(&"video-tile--hero"));
  }

  #[test]
  fn local_tile_adds_local_modifier() {
    let parts = tile_class_parts(false, false, true, true, false);
    assert!(parts.contains(&"video-tile--local"));
  }

  #[test]
  fn no_stream_adds_no_stream_modifier() {
    let parts = tile_class_parts(false, false, false, false, false);
    assert!(parts.contains(&"video-tile--no-stream"));
  }

  #[test]
  fn speaking_adds_speaking_modifier() {
    let parts = tile_class_parts(false, true, false, true, false);
    assert!(parts.contains(&"video-tile--speaking"));
  }

  #[test]
  fn reconnecting_only_applies_to_remote_tiles() {
    // Local tile: reconnecting flag is ignored.
    let local = tile_class_parts(false, false, true, true, true);
    assert!(!local.contains(&"video-tile--reconnecting"));
    // Remote tile: flag surfaces as a modifier.
    let remote = tile_class_parts(false, false, false, true, true);
    assert!(remote.contains(&"video-tile--reconnecting"));
  }

  #[test]
  fn pip_role_prefers_hero_over_local() {
    assert_eq!(pip_candidate_role(true, true), "hero");
    assert_eq!(pip_candidate_role(true, false), "hero");
    assert_eq!(pip_candidate_role(false, true), "local");
    assert_eq!(pip_candidate_role(false, false), "peer");
  }

  #[test]
  fn remote_media_icons_visible_only_on_remote_tiles() {
    assert!(should_show_remote_media_icons(false));
    assert!(!should_show_remote_media_icons(true));
  }
}
