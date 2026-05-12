//! Sidebar section component.
//!
//! Renders a labelled group of conversations with an optional
//! collapse toggle. The collapse mode is used by the "Archived"
//! section per Req 7.7f — archived conversations live below the
//! main list and stay hidden by default so they do not crowd active
//! chats.

use super::sidebar_conversation_item::SidebarConversationItem;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

/// Sidebar section component.
///
/// `collapsible` defaults to `false` (always-expanded). When set to
/// `true` the section header doubles as a button that toggles the
/// list visibility, with the open state persisted in `expanded`.
#[component]
pub fn SidebarSection(
  #[prop(into)] title: Signal<String>,
  conversations: Signal<Vec<crate::state::Conversation>>,
  /// Stable identifier used for `data-testid="sidebar-section-<kind>"`
  /// and as a `data-section` attribute on contained conversation rows.
  /// Tests target sections by this string instead of the localised
  /// title. Defaults to `"unnamed"` for back-compat with any future
  /// callers.
  #[prop(optional, into)]
  kind: Option<&'static str>,
  /// Whether this section can be collapsed by the user. When `false`
  /// (the default) the section is always rendered in the open state
  /// and the header is plain text.
  #[prop(optional)]
  collapsible: bool,
  /// External signal storing the open / collapsed state. Only used
  /// when `collapsible` is `true`. Owned by the parent so the choice
  /// can be persisted (e.g. across re-renders).
  #[prop(optional)]
  expanded: Option<RwSignal<bool>>,
) -> impl IntoView {
  // Hide the section entirely when there's nothing to show — keeping
  // an empty header creates visual clutter, especially in the
  // "Pinned" / "Archived" rows.
  let visible = Signal::derive(move || !conversations.get().is_empty());

  let expanded_signal = expanded.unwrap_or_else(|| RwSignal::new(false));
  let is_open = move || !collapsible || expanded_signal.get();
  let kind_attr = kind.unwrap_or("unnamed");

  view! {
    <Show when=move || visible.get() fallback=|| ()>
      <div
        class="sidebar-section"
        class:sidebar-section--collapsed=move || collapsible && !expanded_signal.get()
        data-testid=format!("sidebar-section-{kind_attr}")
        data-section=kind_attr
      >
        {move || {
          if collapsible {
            let open = expanded_signal.get();
            let count = conversations.get().len();
            view! {
              <button
                type="button"
                class="sidebar-section-title sidebar-section-title--toggle"
                aria-expanded=move || if expanded_signal.get() { "true" } else { "false" }
                on:click=move |_| expanded_signal.update(|v| *v = !*v)
              >
                <Icon
                  icon=if open { i::LuChevronDown } else { i::LuChevronRight }
                  attr:class="sidebar-section-toggle-icon"
                />
                <span>{title}</span>
                <span class="sidebar-section-count">{format!("{count}")}</span>
              </button>
            }
              .into_any()
          } else {
            view! { <div class="sidebar-section-title">{title}</div> }.into_any()
          }
        }}
        <Show when=is_open fallback=|| ()>
          <For
            each=move || conversations.get()
            key=|conv| conv.id.clone()
            children=move |conv| {
              view! { <SidebarConversationItem conversation=conv.clone() /> }
            }
          />
        </Show>
      </div>
    </Show>
  }
}
