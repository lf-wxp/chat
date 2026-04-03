//! Input component

use leptos::prelude::*;

/// Input type variants
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum InputType {
  #[default]
  Text,
  Password,
  Search,
}

/// General input component
#[component]
pub fn Input(
  /// Placeholder text
  #[prop(into, optional)]
  placeholder: String,
  /// Input type
  #[prop(optional)]
  input_type: InputType,
  /// Bound value
  value: RwSignal<String>,
  /// Label text
  #[prop(into, optional)]
  label: String,
  /// Whether disabled
  #[prop(optional)]
  disabled: bool,
  /// Enter key callback
  #[prop(optional)]
  on_enter: Option<Callback<String>>,
) -> impl IntoView {
  let type_attr = match input_type {
    InputType::Text => "text",
    InputType::Password => "password",
    InputType::Search => "search",
  };

  let handle_input = move |ev: web_sys::Event| {
    let target = event_target::<web_sys::HtmlInputElement>(&ev);
    value.set(target.value());
  };

  let handle_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == "Enter"
      && let Some(cb) = &on_enter
    {
      cb.run(value.get_untracked());
    }
  };

  view! {
    <div class="input-group">
      {if label.is_empty() {
        let _: () = view! {};
        ().into_any()
      } else {
        view! { <label class="input-label">{label.clone()}</label> }.into_any()
      }}
      <input
        class="input"
        type=type_attr
        placeholder=placeholder
        disabled=disabled
        prop:value=move || value.get()
        on:input=handle_input
        on:keydown=handle_keydown
        tabindex=0
      />
    </div>
  }
}
