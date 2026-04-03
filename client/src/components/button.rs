//! Button component

use leptos::prelude::*;

/// Button variants
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ButtonVariant {
  #[default]
  Primary,
  Secondary,
  Ghost,
  Danger,
}

/// General button component
#[component]
pub fn Button(
  /// Button label
  #[prop(into)]
  label: String,
  /// Button variant
  #[prop(optional)]
  variant: ButtonVariant,
  /// Whether the button is disabled
  #[prop(optional)]
  disabled: bool,
  /// Whether the button is loading
  #[prop(optional)]
  loading: bool,
  /// Whether the button takes full width
  #[prop(optional)]
  full_width: bool,
  /// Click callback
  #[prop(optional)]
  on_click: Option<Callback<()>>,
) -> impl IntoView {
  let variant_class = match variant {
    ButtonVariant::Primary => "btn-primary",
    ButtonVariant::Secondary => "btn-secondary",
    ButtonVariant::Ghost => "btn-ghost",
    ButtonVariant::Danger => "btn-danger",
  };

  let handle_click = move |_| {
    if !disabled
      && !loading
      && let Some(cb) = &on_click
    {
      cb.run(());
    }
  };

  view! {
    <button
      class=format!("btn {} {}", variant_class, if full_width { "w-full" } else { "" })
      disabled=disabled || loading
      on:click=handle_click
      aria-label=label.clone()
    >
      {if loading {
        view! { <span class="btn-spinner"></span> }.into_any()
      } else {
        view! { <span>{label.clone()}</span> }.into_any()
      }}
    </button>
  }
}
