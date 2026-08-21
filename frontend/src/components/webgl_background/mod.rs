//! Unified WebGL background renderer.
//!
//! Renders three effects into a single full-viewport canvas:
//!
//! 1. **Gradient Waves** — raymarched plasma heightfield (port of
//!    React Bits' `GradientWaves`).
//! 2. **Light Rays** — volumetric light beams from a top anchor
//!    (port of React Bits' `LightRays`).
//! 3. **Particles** — drifting dust motes rendered as GL point
//!    sprites with a radial-gradient falloff.
//!
//! ## Module layout
//!
//! * [`self`] — the [`WebGlBackground`] Leptos component, which owns
//!   the canvas ref and forwards reactive settings to the renderer.
//! * [`renderer`] (wasm32 only) — the actual WebGL2 state machine:
//!   shader compilation, the RAF loop, and per-frame uniform
//!   uploads. See its module docs for the single-instance
//!   architecture rationale (why switching effects doesn't
//!   re-initialize the GL context).
//! * `renderer::shaders` — the raw GLSL source for all three
//!   effects, kept separate from the Rust-side GL plumbing so each
//!   file stays focused and scannable.
//!
//! Respects `prefers-reduced-motion`: renders one static frame and
//! never starts the loop.

use leptos::prelude::*;

use crate::settings::{BackgroundEffects, WaveConfig};

#[cfg(target_arch = "wasm32")]
mod renderer;

/// Setter closure returned by the renderer once initialized: given a
/// new [`BackgroundEffects`] selection plus the current
/// [`WaveConfig`], switches the running WebGL instance to draw that
/// configuration without re-initializing anything.
type EffectSetter = std::rc::Rc<dyn Fn(BackgroundEffects, WaveConfig)>;

#[component]
pub fn WebGlBackground(
  effects: Signal<BackgroundEffects>,
  /// Gradient Waves shader configuration (see
  /// `BackgroundSettings::waves`). Ignored by the Rays/Particles
  /// programs.
  waves: Signal<WaveConfig>,
) -> impl IntoView {
  let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

  // Holds the setter returned by `renderer::start()` once the
  // renderer has been initialized. Typed as a boxed `dyn Fn` (rather
  // than the wasm-only `renderer` module's concrete types) so this
  // `StoredValue` is valid on every compile target, including SSR.
  // `LocalStorage` is required because `Rc<dyn Fn>` is not `Send`.
  let setter: StoredValue<Option<EffectSetter>, LocalStorage> = StoredValue::new_local(None);

  // Initialize the renderer exactly once, when the canvas mounts.
  #[cfg(target_arch = "wasm32")]
  Effect::new(move |_| {
    let Some(canvas) = canvas_ref.get() else {
      return;
    };
    if setter.get_value().is_some() {
      return;
    }
    let initial = effects.get_untracked();
    let initial_waves = waves.get_untracked();
    let handle = renderer::start(canvas, initial, initial_waves);
    setter.set_value(Some(handle));
  });

  // Whenever the selected effect or wave knobs change, forward them
  // to the already-running renderer instead of restarting everything.
  Effect::new(move |_| {
    let fx = effects.get();
    let wv = waves.get();
    if let Some(handle) = setter.get_value() {
      handle(fx, wv);
    }
  });

  view! {
    <canvas
      node_ref=canvas_ref
      class="webgl-background"
      aria-hidden="true"
      data-testid="webgl-background"
    />
  }
}
