//! # client
//!
//! WebRTC chat application frontend (Leptos CSR + WASM).

// Leptos `view! {}` macro generates `unused_unit` warning for empty views, which is framework behavior and unavoidable
#![allow(clippy::unused_unit)]

mod app;
mod call;
mod chat;
mod components;
mod crypto;
mod flow_control;
pub mod i18n;
mod network_quality;
mod pages;
mod pip;
mod services;
mod state;
mod sticker;
mod storage;
mod theater;
mod transfer;
mod utils;
mod vad;

fn main() {
  // Initialize console_error_panic_hook for better error messages in WASM
  console_error_panic_hook::set_once();

  leptos::mount::mount_to_body(app::App);
}
