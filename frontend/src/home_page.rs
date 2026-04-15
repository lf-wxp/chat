//! Home page component.

use crate::i18n;
use leptos::prelude::*;
use leptos_i18n::t;

/// Home page component.
#[component]
pub fn HomePage() -> impl IntoView {
  let i18n = i18n::use_i18n();
  view! {
    <div class="flex items-center justify-center h-full">
      <div class="text-center p-8">
        <h1 class="text-2xl font-bold mb-4">{t!(i18n, app.title)}</h1>
        <p class="text-secondary">{t!(i18n, app.welcome)}</p>
      </div>
    </div>
  }
}
