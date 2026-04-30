//! Canvas-based voice waveform renderer for message bubbles.
//!
//! Renders the waveform amplitude samples as a smooth bar chart on a
//! `<canvas>` element. Supports a progress fill that highlights the
//! portion of the waveform that has already been played.
//!
//! The canvas automatically adapts its logical dimensions to the
//! available parent width via a `ResizeObserver`, so it never overflows
//! the message bubble regardless of clip duration.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Canvas logical height for the inline message waveform.
const CANVAS_HEIGHT: u32 = 40;

/// Gap between bars in logical pixels.
const BAR_GAP: f64 = 2.0;

/// Minimum bar height in logical pixels (so silent samples are still
/// visible).
const MIN_BAR_H: f64 = 2.0;

/// Corner radius for each bar (rounded caps).
const BAR_RADIUS: f64 = 1.5;

/// Canvas-based voice waveform component.
///
/// Renders a static waveform from the `bars` samples (0..=255) and
/// optionally highlights the `progress` fraction (0.0..=1.0) in a
/// contrasting colour to indicate playback position.
#[component]
pub fn VoiceWaveform(
  /// Waveform amplitude samples normalised to 0..=255.
  bars: Vec<u8>,
  /// Whether this bubble is outgoing (affects colour scheme).
  #[prop(optional)]
  outgoing: bool,
  /// Playback progress as a fraction 0.0..=1.0.
  #[prop(optional)]
  progress: Signal<f64>,
) -> impl IntoView {
  let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
  let bars_clone = bars.clone();
  let outgoing_clone = outgoing;

  // ResizeObserver: keep the canvas logical size in sync with its
  // CSS layout size so the drawing is always crisp and fits the
  // parent container without overflow.
  Effect::new(move |_| {
    let Some(canvas_el) = canvas_ref.get() else {
      return;
    };
    let html_canvas: &web_sys::HtmlCanvasElement = canvas_el.as_ref();

    // Initial draw with current dimensions.
    let progress_val = progress.get();
    draw_waveform(html_canvas, &bars_clone, outgoing_clone, progress_val);

    // Observe size changes and redraw when the container resizes.
    let bars_for_observer = bars_clone.clone();
    let outgoing_for_observer = outgoing_clone;
    let progress_for_observer = progress;
    let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |entries: js_sys::Array| {
      if let Some(entry) = entries.get(0).dyn_ref::<web_sys::ResizeObserverEntry>() {
        let rect = entry.content_rect();
        let new_w = rect.width() as u32;
        if new_w == 0 {
          return;
        }
        let target = entry.target();
        let Ok(canvas) = target.dyn_into::<web_sys::HtmlCanvasElement>() else {
          return;
        };
        // Only update + redraw if the width actually changed.
        if canvas.width() != new_w {
          canvas.set_width(new_w);
          canvas.set_height(CANVAS_HEIGHT);
          let p = progress_for_observer.get();
          draw_waveform(&canvas, &bars_for_observer, outgoing_for_observer, p);
        }
      }
    }) as Box<dyn FnMut(js_sys::Array)>);

    let observer = match web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
      Ok(o) => o,
      Err(_) => return,
    };
    let _ = cb.into_js_value();
    observer.observe(html_canvas);

    on_cleanup(move || {
      observer.disconnect();
    });
  });

  // Redraw on progress changes.
  let bars_for_progress = bars.clone();
  let outgoing_for_progress = outgoing;
  Effect::new(move |_| {
    let Some(canvas_el) = canvas_ref.get() else {
      return;
    };
    let html_canvas: &web_sys::HtmlCanvasElement = canvas_el.as_ref();
    let progress_val = progress.get();
    draw_waveform(
      html_canvas,
      &bars_for_progress,
      outgoing_for_progress,
      progress_val,
    );
  });

  view! {
    <canvas
      node_ref=canvas_ref
      class="voice-waveform-canvas"
      height=CANVAS_HEIGHT
      aria-hidden="true"
    />
  }
}

/// Draw the waveform bars on the canvas.
///
/// * `canvas` — the target `<canvas>` element.
/// * `bars` — amplitude samples 0..=255.
/// * `outgoing` — selects the colour palette (blue bubble vs grey).
/// * `progress` — fraction 0.0..=1.0 of played audio.
fn draw_waveform(canvas: &web_sys::HtmlCanvasElement, bars: &[u8], outgoing: bool, progress: f64) {
  let Ok(Some(ctx_js)) = canvas.get_context("2d") else {
    return;
  };
  let Ok(ctx) = ctx_js.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
    return;
  };

  let w = f64::from(canvas.width());
  let h = f64::from(canvas.height());
  ctx.clear_rect(0.0, 0.0, w, h);

  if bars.is_empty() || w <= 0.0 {
    return;
  }

  // Normalise to percentage heights.
  let max = bars.iter().copied().max().unwrap_or(1).max(1);
  let bar_width = (w / bars.len() as f64 - BAR_GAP).max(2.0);
  let progress_x = w * progress.clamp(0.0, 1.0);

  for (i, &sample) in bars.iter().enumerate() {
    let pct = (f64::from(sample) / f64::from(max)).clamp(0.08, 1.0);
    let bar_h = (pct * h).max(MIN_BAR_H);
    let x = i as f64 * (bar_width + BAR_GAP);
    let y = (h - bar_h) / 2.0;

    // Choose colour based on outgoing/incoming and progress position.
    let is_played = x + bar_width / 2.0 <= progress_x;
    let colour = if outgoing {
      if is_played {
        "rgba(255,255,255,1.0)"
      } else {
        "rgba(255,255,255,0.5)"
      }
    } else if is_played {
      "var(--color-primary, #3b82f6)"
    } else {
      "var(--text-tertiary, #94a3b8)"
    };

    ctx.set_fill_style_str(colour);
    draw_rounded_rect(&ctx, x, y, bar_width, bar_h, BAR_RADIUS);
  }
}

/// Draw a rounded rectangle (or plain rect if radius is negligible).
fn draw_rounded_rect(
  ctx: &web_sys::CanvasRenderingContext2d,
  x: f64,
  y: f64,
  w: f64,
  h: f64,
  r: f64,
) {
  if r <= 0.0 || w < 2.0 * r || h < 2.0 * r {
    ctx.fill_rect(x, y, w, h);
    return;
  }
  ctx.begin_path();
  ctx.move_to(x + r, y);
  ctx.line_to(x + w - r, y);
  ctx
    .arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0)
    .unwrap_or_default();
  ctx.line_to(x + w, y + h - r);
  ctx
    .arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2)
    .unwrap_or_default();
  ctx.line_to(x + r, y + h);
  ctx
    .arc(
      x + r,
      y + h - r,
      r,
      std::f64::consts::FRAC_PI_2,
      std::f64::consts::PI,
    )
    .unwrap_or_default();
  ctx.line_to(x, y + r);
  ctx
    .arc(
      x + r,
      y + r,
      r,
      std::f64::consts::PI,
      1.5 * std::f64::consts::PI,
    )
    .unwrap_or_default();
  ctx.close_path();
  ctx.fill();
}
