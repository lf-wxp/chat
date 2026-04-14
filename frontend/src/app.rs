//! Main application component.

use leptos::prelude::*;

/// Main application component.
#[component]
fn App() -> impl IntoView {
  view! {
    <div class="app">
      <h1>"WebRTC Chat"</h1>
      <p>"Welcome to WebRTC Chat Application"</p>
    </div>
  }
}
