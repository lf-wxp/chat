//! Theater copyright notice (Req 12.1).
//!
//! Rendered next to the "Create Theater" form controls and next to the
//! video-source picker so the viewer always has the copyright disclaimer
//! at hand. The component is purely informational — it uses an info
//! icon with a tooltip instead of a dismissable banner, matching the
//! "non-intrusive manner" wording in the acceptance criteria.

use icondata as i;
use leptos::prelude::*;
use leptos_i18n::t_string;
use leptos_icons::Icon;

use crate::i18n;

/// Non-intrusive copyright notice icon + tooltip.
///
/// The tooltip expands on hover / focus so keyboard users can also
/// read the disclaimer. When `inline` is `true` the component renders
/// the short label beside the icon so the notice is visible without
/// any hover interaction (used inside the create-theater form).
#[component]
pub fn CopyrightNotice(
  /// When `true`, render the short "Copyright" label next to the icon.
  #[prop(optional)]
  inline: bool,
) -> impl IntoView {
  let i18n = i18n::use_i18n();
  let tooltip_text = move || t_string!(i18n, theater.copyright_notice).to_string();
  let short_label = move || t_string!(i18n, theater.copyright_notice_short).to_string();

  view! {
    <span
      class="theater-copyright"
      tabindex="0"
      role="note"
      aria-label=tooltip_text
      data-testid="theater-copyright-notice"
    >
      <Icon icon=i::LuInfo />
      <Show when=move || inline>
        <span class="theater-copyright__label">{short_label}</span>
      </Show>
      <span class="theater-copyright__tooltip" role="tooltip">
        {tooltip_text}
      </span>
    </span>
  }
}
