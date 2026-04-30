//! Mute duration picker modal.
//!
//! Presents the caller with the Req 15.3 §20 predefined durations
//! (1 min, 5 min, 30 min, 1 hr, permanent) and emits the chosen
//! duration back to the parent component.

use leptos::prelude::*;
use leptos_i18n::t_string;

use crate::components::room::modal_wrapper::{ModalSize, ModalWrapper};
use crate::components::room::utils::interpolate_name;
use crate::i18n;

/// A single mute duration option. `None` represents a permanent mute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DurationOption {
  /// Label lookup key on the i18n tree.
  label_secs: Option<u64>,
}

/// Predefined mute durations in seconds (or `None` for permanent mute).
const DURATIONS: &[DurationOption] = &[
  DurationOption {
    label_secs: Some(60),
  },
  DurationOption {
    label_secs: Some(5 * 60),
  },
  DurationOption {
    label_secs: Some(30 * 60),
  },
  DurationOption {
    label_secs: Some(60 * 60),
  },
  DurationOption { label_secs: None },
];

/// Mute duration picker modal.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn MuteDurationPicker(
  /// Display name of the target member (used in the modal title).
  #[prop(into)]
  target_name: Signal<String>,
  /// Fires with `Some(seconds)` for a timed mute, `None` for permanent.
  on_pick: Callback<Option<u64>>,
  /// Fires when the user cancels the modal (button, backdrop, Escape).
  on_cancel: Callback<()>,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  view! {
    <ModalWrapper
      on_close=on_cancel
      size=ModalSize::Small
      class="mute-duration-picker"
      labelled_by="mute-duration-title"
      testid="mute-duration-picker"
    >
      <header class="modal-header">
        <h2 id="mute-duration-title" class="modal-title">
          {move || {
            let template = t_string!(i18n, room.mute_title);
            interpolate_name(template, &target_name.get())
          }}
        </h2>
      </header>
      <div class="modal-body mute-duration-picker__body">
        {move || {
          DURATIONS
            .iter()
            .copied()
            .map(|opt| {
              let label = match opt.label_secs {
                Some(60) => t_string!(i18n, room.mute_duration_1m).to_string(),
                Some(300) => t_string!(i18n, room.mute_duration_5m).to_string(),
                Some(1800) => t_string!(i18n, room.mute_duration_30m).to_string(),
                Some(3600) => t_string!(i18n, room.mute_duration_1h).to_string(),
                None => t_string!(i18n, room.mute_duration_permanent).to_string(),
                Some(_) => unreachable!("DURATIONS constant is exhaustive"),
              };
              let aria_label = match opt.label_secs {
                Some(60) => t_string!(i18n, room.mute_duration_1m_aria).to_string(),
                Some(300) => t_string!(i18n, room.mute_duration_5m_aria).to_string(),
                Some(1800) => t_string!(i18n, room.mute_duration_30m_aria).to_string(),
                Some(3600) => t_string!(i18n, room.mute_duration_1h_aria).to_string(),
                None => t_string!(i18n, room.mute_duration_permanent_aria).to_string(),
                Some(_) => unreachable!("DURATIONS constant is exhaustive"),
              };
              view! {
                <button
                  type="button"
                  class="btn btn--ghost mute-duration-picker__option"
                  aria-label=aria_label
                  data-testid="mute-duration-option"
                  on:click=move |_| on_pick.run(opt.label_secs)
                >
                  {label}
                </button>
              }
            })
            .collect_view()
        }}
      </div>
      <footer class="modal-footer">
        <button
          type="button"
          class="btn btn--ghost"
          on:click=move |_| on_cancel.run(())
          data-testid="mute-duration-cancel"
        >
          {t_string!(i18n, common.cancel)}
        </button>
      </footer>
    </ModalWrapper>
  }
}
