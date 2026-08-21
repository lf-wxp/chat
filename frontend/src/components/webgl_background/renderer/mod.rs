//! WebGL2 state machine driving the background canvas.
//!
//! ## Single-instance architecture
//!
//! [`start`] runs **exactly once** per page load: it creates the
//! WebGL2 context, compiles *all three* shader programs up front
//! (regardless of which effect is initially selected), and spins up
//! one RAF loop + one set of `resize`/`mousemove`/
//! `visibilitychange` listeners.
//!
//! Switching effects does **not** call `start()` again — that used
//! to leak a brand-new context/loop/listener set on every switch,
//! leaving old RAF loops running forever in the background (racing
//! with the new one for the same canvas, which is also why only one
//! effect ever appeared to render). Instead, `start()` returns a
//! setter closure that just flips a `BackgroundEffects` field on the
//! shared render state; [`draw_frame`] reads that field every frame
//! to decide which program(s) to draw.
//!
//! Selecting `None` stops the RAF loop entirely (no more frames are
//! scheduled) instead of looping forever doing nothing. Selecting a
//! real effect again restarts the loop from the setter.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
  HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlProgram, WebGlShader, WebGlUniformLocation,
  WebGlVertexArrayObject,
};

use super::{BackgroundEffects, WaveConfig};

mod shaders;
use shaders::{FRAG_PARTICLES, FRAG_RAYS, FRAG_WAVES, VERT, VERT_PARTICLES};

// ── GL helpers ─────────────────────────────────────────────────

fn compile_shader(gl: &GL, kind: u32, src: &str) -> Option<WebGlShader> {
  let shader = gl.create_shader(kind)?;
  gl.shader_source(&shader, src);
  gl.compile_shader(&shader);
  let ok = gl
    .get_shader_parameter(&shader, GL::COMPILE_STATUS)
    .as_bool()
    .unwrap_or(false);
  if !ok {
    let log = gl.get_shader_info_log(&shader).unwrap_or_default();
    web_sys::console::error_1(&format!("[WebGL] shader compile error: {log}").into());
    return None;
  }
  Some(shader)
}

fn link_program(gl: &GL, vs: &WebGlShader, fs: &WebGlShader) -> Option<WebGlProgram> {
  let prog = gl.create_program()?;
  gl.attach_shader(&prog, vs);
  gl.attach_shader(&prog, fs);
  gl.link_program(&prog);
  let ok = gl
    .get_program_parameter(&prog, GL::LINK_STATUS)
    .as_bool()
    .unwrap_or(false);
  if !ok {
    let log = gl.get_program_info_log(&prog).unwrap_or_default();
    web_sys::console::error_1(&format!("[WebGL] program link error: {log}").into());
    return None;
  }
  Some(prog)
}

fn hex_to_rgb(hex: &str) -> [f32; 3] {
  let hex = hex.trim_start_matches('#');
  if hex.len() != 6 {
    return [1.0, 1.0, 1.0];
  }
  let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
  let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f32 / 255.0;
  let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f32 / 255.0;
  [r, g, b]
}

fn fullscreen_triangle_vao(gl: &GL, program: &WebGlProgram) -> Option<WebGlVertexArrayObject> {
  let vao = gl.create_vertex_array()?;
  gl.bind_vertex_array(Some(&vao));
  let buf = gl.create_buffer()?;
  gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
  let verts: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];
  unsafe {
    let arr = js_sys::Float32Array::view(&verts);
    gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &arr, GL::STATIC_DRAW);
  }
  let loc = gl.get_attrib_location(program, "position");
  gl.enable_vertex_attrib_array(loc as u32);
  gl.vertex_attrib_pointer_with_i32(loc as u32, 2, GL::FLOAT, false, 0, 0);
  Some(vao)
}

// ── Renderer state ─────────────────────────────────────────────

struct Effect {
  program: WebGlProgram,
  vao: WebGlVertexArrayObject,
  uniforms: UniformLocs,
}

#[derive(Default)]
struct UniformLocs {
  i_resolution: Option<WebGlUniformLocation>,
  i_time: Option<WebGlUniformLocation>,
  u_mouse: Option<WebGlUniformLocation>,
  u_opacity: Option<WebGlUniformLocation>,
  u_horizon_color: Option<WebGlUniformLocation>,
  u_wave_color: Option<WebGlUniformLocation>,
  u_crest_color: Option<WebGlUniformLocation>,
  u_wave_scale: Option<WebGlUniformLocation>,
  u_wave_ratio: Option<WebGlUniformLocation>,
  u_wave_speed: Option<WebGlUniformLocation>,
  u_wave_swell: Option<WebGlUniformLocation>,
  u_wave_turbulence: Option<WebGlUniformLocation>,
  u_wave_tilt: Option<WebGlUniformLocation>,
  u_wave_zoom: Option<WebGlUniformLocation>,
  u_wave_horizon_height: Option<WebGlUniformLocation>,
  u_wave_fog_depth: Option<WebGlUniformLocation>,
  u_wave_brightness: Option<WebGlUniformLocation>,
  u_rays_color: Option<WebGlUniformLocation>,
  u_color: Option<WebGlUniformLocation>,
  u_dpr: Option<WebGlUniformLocation>,
}

struct RenderState {
  gl: GL,
  waves: Option<Effect>,
  rays: Option<Effect>,
  particles: Option<(Effect, usize)>,
  mouse: Rc<RefCell<(f32, f32)>>,
  smooth_mouse: Rc<RefCell<(f32, f32)>>,
  /// Tab-visibility gate — paused while the tab is hidden.
  visible: bool,
  /// Which effect(s) to draw this frame. Updated by the setter
  /// closure returned from [`start`] whenever the user switches
  /// effects in Settings.
  active: BackgroundEffects,
  /// Gradient Waves shader configuration — see
  /// `BackgroundSettings::waves`. Updated live by the setter
  /// closure whenever the user drags a Settings slider.
  waves_config: WaveConfig,
  start_time: f64,
}

/// Per-theme colour + opacity recipe for all three effects.
///
/// The canvas composites onto the page with the browser's default
/// alpha-over (`Co = Cs·α + Cb·(1-α)`) — no `mix-blend-mode` is
/// used (see `background.css`). `screen` and `multiply` blending
/// were both tried and measured via `gl.readPixels` to be nearly
/// invisible on at least one theme (screen(bright, pale) ≈ pale on
/// light backgrounds; multiply barely darkens at the alpha ranges
/// these shaders actually produce). With plain alpha-over,
/// visibility depends only on alpha and color contrast against the
/// theme's backdrop, so:
///
/// * **Dark** theme: bright, saturated "glow" colors (near-white /
///   sky blue) read clearly against the near-black canvas.
/// * **Light** theme: deep, saturated "ink" colors (royal blue /
///   slate) read clearly against the pale canvas.
struct Palette {
  waves_horizon: [f32; 3],
  waves_body: [f32; 3],
  waves_crest: [f32; 3],
  waves_opacity: f32,
  rays_color: [f32; 3],
  rays_opacity: f32,
  particles_color: [f32; 3],
  particles_opacity: f32,
}

impl Palette {
  fn for_theme(is_dark: bool) -> Self {
    if is_dark {
      Self {
        waves_horizon: hex_to_rgb("#0a0e1a"),
        waves_body: hex_to_rgb("#4f8ff7"),
        waves_crest: hex_to_rgb("#93c5fd"),
        waves_opacity: 0.6,
        rays_color: hex_to_rgb("#93c5fd"),
        rays_opacity: 0.65,
        particles_color: hex_to_rgb("#ffffff"),
        particles_opacity: 0.7,
      }
    } else {
      Self {
        waves_horizon: hex_to_rgb("#ffffff"),
        waves_body: hex_to_rgb("#2563eb"),
        waves_crest: hex_to_rgb("#1d4ed8"),
        waves_opacity: 0.55,
        rays_color: hex_to_rgb("#1d4ed8"),
        rays_opacity: 0.6,
        particles_color: hex_to_rgb("#1e293b"),
        particles_opacity: 0.55,
      }
    }
  }
}

/// Reads `<html data-theme="...">`, defaulting to dark when absent
/// (matches the app's own default in `app.rs`).
fn is_dark_theme() -> bool {
  web_sys::window()
    .and_then(|w| w.document())
    .and_then(|d| d.document_element())
    .and_then(|el| el.get_attribute("data-theme"))
    .map(|t| t != "light")
    .unwrap_or(true)
}

/// Initializes the WebGL2 renderer exactly once and returns a
/// setter closure that switches the active effect (and Gradient
/// Waves knobs) on the already-running instance (no
/// re-initialization, no leaked contexts).
pub fn start(
  canvas: HtmlCanvasElement,
  initial_effects: BackgroundEffects,
  initial_waves: WaveConfig,
) -> super::EffectSetter {
  // A no-op setter used for every early-return error path so the
  // component always gets a valid handle back, even if WebGL2 is
  // unsupported or a shader fails to compile.
  let noop: super::EffectSetter = Rc::new(|_, _| {});

  let Some(window) = web_sys::window() else {
    return noop;
  };

  let reduced_motion = window
    .match_media("(prefers-reduced-motion: reduce)")
    .ok()
    .flatten()
    .map(|mq| mq.matches())
    .unwrap_or(false);

  // `premultipliedAlpha: false` is required because our shaders
  // output *straight* (non-premultiplied) alpha — the blend func
  // below does `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`, which assumes
  // straight alpha. WebGL defaults `premultipliedAlpha` to `true`,
  // which makes the browser composite the canvas onto the page as
  // if our colors were already alpha-multiplied. With straight
  // alpha output and the default `true`, every effect renders at
  // roughly `alpha²` brightness instead of `alpha` — which is why
  // all three effects were nearly invisible even though the WebGL
  // draw calls themselves were correct (canvas size, active
  // effect, and draw branches all checked out in the console).
  let ctx_options = js_sys::Object::new();
  js_sys::Reflect::set(
    &ctx_options,
    &"premultipliedAlpha".into(),
    &wasm_bindgen::JsValue::FALSE,
  )
  .ok();
  js_sys::Reflect::set(&ctx_options, &"alpha".into(), &wasm_bindgen::JsValue::TRUE).ok();

  let Some(gl) = canvas
    .get_context_with_context_options("webgl2", &ctx_options)
    .ok()
    .flatten()
    .and_then(|c| c.dyn_into::<GL>().ok())
  else {
    web_sys::console::warn_1(&"[WebGL] WebGL2 not supported".into());
    return noop;
  };

  // Standard (non-premultiplied) alpha blending — our shaders all
  // output straight alpha.
  gl.enable(GL::BLEND);
  gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

  let Some(vs) = compile_shader(&gl, GL::VERTEX_SHADER, VERT) else {
    return noop;
  };

  // Compile ALL THREE effects unconditionally, regardless of which
  // one is initially selected — switching effects later only flips
  // `RenderState::active`, so every program must already exist.
  let waves = (|| {
    let fs = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAG_WAVES)?;
    let program = link_program(&gl, &vs, &fs)?;
    let vao = fullscreen_triangle_vao(&gl, &program)?;
    let uniforms = UniformLocs {
      i_resolution: gl.get_uniform_location(&program, "iResolution"),
      i_time: gl.get_uniform_location(&program, "iTime"),
      u_mouse: gl.get_uniform_location(&program, "uMouse"),
      u_opacity: gl.get_uniform_location(&program, "uOpacity"),
      u_horizon_color: gl.get_uniform_location(&program, "uHorizonColor"),
      u_wave_color: gl.get_uniform_location(&program, "uWaveColor"),
      u_crest_color: gl.get_uniform_location(&program, "uCrestColor"),
      u_wave_scale: gl.get_uniform_location(&program, "uWaveScale"),
      u_wave_ratio: gl.get_uniform_location(&program, "uWaveRatio"),
      u_wave_speed: gl.get_uniform_location(&program, "uWaveSpeed"),
      u_wave_swell: gl.get_uniform_location(&program, "uWaveSwell"),
      u_wave_turbulence: gl.get_uniform_location(&program, "uWaveTurbulence"),
      u_wave_tilt: gl.get_uniform_location(&program, "uWaveTilt"),
      u_wave_zoom: gl.get_uniform_location(&program, "uWaveZoom"),
      u_wave_horizon_height: gl.get_uniform_location(&program, "uWaveHorizonHeight"),
      u_wave_fog_depth: gl.get_uniform_location(&program, "uWaveFogDepth"),
      u_wave_brightness: gl.get_uniform_location(&program, "uWaveBrightness"),
      ..Default::default()
    };
    Some(Effect {
      program,
      vao,
      uniforms,
    })
  })();

  let rays = (|| {
    let fs = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAG_RAYS)?;
    let program = link_program(&gl, &vs, &fs)?;
    let vao = fullscreen_triangle_vao(&gl, &program)?;
    let uniforms = UniformLocs {
      i_resolution: gl.get_uniform_location(&program, "iResolution"),
      i_time: gl.get_uniform_location(&program, "iTime"),
      u_mouse: gl.get_uniform_location(&program, "uMouse"),
      u_opacity: gl.get_uniform_location(&program, "uOpacity"),
      u_rays_color: gl.get_uniform_location(&program, "uRaysColor"),
      ..Default::default()
    };
    Some(Effect {
      program,
      vao,
      uniforms,
    })
  })();

  let particles = (|| {
    let vs_p = compile_shader(&gl, GL::VERTEX_SHADER, VERT_PARTICLES)?;
    let fs_p = compile_shader(&gl, GL::FRAGMENT_SHADER, FRAG_PARTICLES)?;
    let program = link_program(&gl, &vs_p, &fs_p)?;
    let vao = gl.create_vertex_array()?;
    gl.bind_vertex_array(Some(&vao));

    let w = window.inner_width().ok()?.as_f64()? as f32;
    let h = window.inner_height().ok()?.as_f64()? as f32;
    // Density raised again from 28/Mpx² (cap 160) to 60/Mpx² (cap
    // 300), and size from 4-9px to 8-16px: even at the previous
    // "fixed" density, the DPR-scaled point sprites still read as
    // a handful of barely-there dots once composited under the
    // Waves gradient at typical `particles_opacity` (~0.55-0.7).
    // The combination of more, bigger dots is what actually
    // survives the `smoothstep` radial falloff and low base alpha.
    let count = ((w * h * 60.0 / 1_000_000.0) as usize).min(300);
    let mut data: Vec<f32> = Vec::with_capacity(count * 4);
    for i in 0..count {
      let seed = i as f32 / count as f32;
      data.push(js_sys::Math::random() as f32 * w);
      data.push(js_sys::Math::random() as f32 * h);
      data.push(8.0 + js_sys::Math::random() as f32 * 8.0);
      data.push(seed * std::f32::consts::TAU);
    }

    let buf = gl.create_buffer()?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buf));
    unsafe {
      let arr = js_sys::Float32Array::view(&data);
      gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &arr, GL::STATIC_DRAW);
    }

    let stride = 4 * 4;
    let pos_loc = gl.get_attrib_location(&program, "aPos");
    gl.enable_vertex_attrib_array(pos_loc as u32);
    gl.vertex_attrib_pointer_with_i32(pos_loc as u32, 2, GL::FLOAT, false, stride, 0);

    let size_loc = gl.get_attrib_location(&program, "aSize");
    gl.enable_vertex_attrib_array(size_loc as u32);
    gl.vertex_attrib_pointer_with_i32(size_loc as u32, 1, GL::FLOAT, false, stride, 8);

    let phase_loc = gl.get_attrib_location(&program, "aPhase");
    gl.enable_vertex_attrib_array(phase_loc as u32);
    gl.vertex_attrib_pointer_with_i32(phase_loc as u32, 1, GL::FLOAT, false, stride, 12);

    let uniforms = UniformLocs {
      i_resolution: gl.get_uniform_location(&program, "iResolution"),
      i_time: gl.get_uniform_location(&program, "iTime"),
      u_color: gl.get_uniform_location(&program, "uColor"),
      u_opacity: gl.get_uniform_location(&program, "uOpacity"),
      u_dpr: gl.get_uniform_location(&program, "uDpr"),
      ..Default::default()
    };
    Some((
      Effect {
        program,
        vao,
        uniforms,
      },
      count,
    ))
  })();

  // ── Mouse tracking ──
  let mouse = Rc::new(RefCell::new((0.5_f32, 0.5_f32)));
  let smooth_mouse = Rc::new(RefCell::new((0.5_f32, 0.5_f32)));
  {
    let mouse = Rc::clone(&mouse);
    let closure = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
      let Some(win) = web_sys::window() else { return };
      let w = win
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1024.0) as f32;
      let h = win
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(768.0) as f32;
      *mouse.borrow_mut() = (ev.client_x() as f32 / w, ev.client_y() as f32 / h);
    }) as Box<dyn FnMut(_)>);
    window
      .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
      .ok();
    closure.forget();
  }

  // ── Resize ──
  let do_resize = {
    let canvas = canvas.clone();
    let gl = gl.clone();
    move || {
      let Some(win) = web_sys::window() else { return };
      let w = win
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
      let h = win
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(600.0);
      let dpr = win.device_pixel_ratio().min(2.0);
      canvas.set_width((w * dpr) as u32);
      canvas.set_height((h * dpr) as u32);
      canvas.style().set_property("width", &format!("{w}px")).ok();
      canvas
        .style()
        .set_property("height", &format!("{h}px"))
        .ok();
      gl.viewport(0, 0, (w * dpr) as i32, (h * dpr) as i32);
    }
  };
  do_resize();
  {
    let resize_cb = do_resize.clone();
    let closure = Closure::wrap(Box::new(resize_cb) as Box<dyn FnMut()>);
    window
      .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
      .ok();
    closure.forget();
  }

  let state = Rc::new(RefCell::new(RenderState {
    gl,
    waves,
    rays,
    particles,
    mouse,
    smooth_mouse,
    visible: true,
    active: initial_effects,
    waves_config: initial_waves,
    start_time: js_sys::Date::now(),
  }));

  // ── Visibility pause ──
  {
    let state = Rc::clone(&state);
    let closure = Closure::wrap(Box::new(move || {
      if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        state.borrow_mut().visible = !doc.hidden();
      }
    }) as Box<dyn FnMut()>);
    if let Some(doc) = window.document() {
      doc
        .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
        .ok();
    }
    closure.forget();
  }

  if reduced_motion {
    draw_frame(&state.borrow());
    return noop;
  }

  // ── RAF driver ──
  //
  // `raf_closure` is a self-referential `Rc<RefCell<Option<Closure>>>`
  // — the closure body captures a clone of `raf_closure` itself so
  // it can re-schedule its own next frame. This creates an
  // intentional reference cycle that keeps the closure alive for
  // the app's lifetime without an explicit `mem::forget`.
  type RafClosure = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;
  let raf_closure: RafClosure = Rc::new(RefCell::new(None));
  // Tracks whether a frame is currently scheduled, so `start_loop`
  // below is idempotent (calling it while already running is a
  // no-op) and the loop can be cleanly stopped when `active` is
  // `None`.
  let loop_running = Rc::new(Cell::new(false));

  {
    let state = Rc::clone(&state);
    let raf_closure_self = Rc::clone(&raf_closure);
    let loop_running_body = Rc::clone(&loop_running);
    *raf_closure.borrow_mut() = Some(Closure::wrap(Box::new(move || {
      let st = state.borrow_mut();
      if st.active == BackgroundEffects::None {
        // Stop the chain entirely — no more frames are scheduled
        // until the setter picks a real effect again. This is the
        // actual fix for "the old loop keeps running after
        // selecting None, wasting resources".
        loop_running_body.set(false);
        return;
      }
      if st.visible {
        let (tx, ty) = *st.mouse.borrow();
        let (cx, cy) = *st.smooth_mouse.borrow();
        let smoothing = 0.92;
        let nx = cx * smoothing + tx * (1.0 - smoothing);
        let ny = cy * smoothing + ty * (1.0 - smoothing);
        *st.smooth_mouse.borrow_mut() = (nx, ny);
        draw_frame(&st);
      }
      drop(st);
      if let Some(cb) = raf_closure_self.borrow().as_ref()
        && let Some(win) = web_sys::window()
      {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
      }
    }) as Box<dyn FnMut()>));
  }

  let start_loop: Rc<dyn Fn()> = {
    let raf_closure = Rc::clone(&raf_closure);
    let loop_running = Rc::clone(&loop_running);
    Rc::new(move || {
      if loop_running.get() {
        return;
      }
      loop_running.set(true);
      if let Some(cb) = raf_closure.borrow().as_ref()
        && let Some(win) = web_sys::window()
      {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
      }
    })
  };

  if initial_effects == BackgroundEffects::None {
    // Paint one cleared frame so no stale content lingers, but
    // don't start the loop — nothing to animate.
    draw_frame(&state.borrow());
  } else {
    start_loop();
  }

  let setter_state = Rc::clone(&state);
  Rc::new(move |fx: BackgroundEffects, wv: WaveConfig| {
    {
      let mut st = setter_state.borrow_mut();
      st.active = fx;
      st.waves_config = wv;
    }
    if fx == BackgroundEffects::None {
      // Clear immediately; the RAF body will see `active == None`
      // on its next tick (already scheduled) and stop rescheduling
      // itself. If the loop had already stopped, this clear is the
      // only paint that happens.
      draw_frame(&setter_state.borrow());
    } else {
      start_loop();
    }
  })
}

fn draw_frame(state: &RenderState) {
  let gl = &state.gl;
  let w = gl.drawing_buffer_width() as f32;
  let h = gl.drawing_buffer_height() as f32;
  let t = ((js_sys::Date::now() - state.start_time) / 1000.0) as f32;
  let (mx, my) = *state.smooth_mouse.borrow();

  // Re-check the theme every frame — it's a cheap DOM attribute
  // read, and it lets the effects react instantly when the user
  // flips the theme toggle without needing a dedicated listener.
  let palette = Palette::for_theme(is_dark_theme());

  gl.clear_color(0.0, 0.0, 0.0, 0.0);
  gl.clear(GL::COLOR_BUFFER_BIT);

  let draw_waves = matches!(
    state.active,
    BackgroundEffects::All | BackgroundEffects::Waves
  );
  let draw_rays = matches!(
    state.active,
    BackgroundEffects::All | BackgroundEffects::Rays
  );
  let draw_particles = matches!(
    state.active,
    BackgroundEffects::All | BackgroundEffects::Particles
  );

  // 1. Gradient Waves
  if draw_waves && let Some(ref fx) = state.waves {
    gl.use_program(Some(&fx.program));
    gl.bind_vertex_array(Some(&fx.vao));
    if let Some(ref u) = fx.uniforms.i_resolution {
      gl.uniform2f(Some(u), w, h);
    }
    if let Some(ref u) = fx.uniforms.i_time {
      gl.uniform1f(Some(u), t);
    }
    if let Some(ref u) = fx.uniforms.u_mouse {
      gl.uniform2f(Some(u), mx, my);
    }
    if let Some(ref u) = fx.uniforms.u_opacity {
      // `opacity` is a user multiplier on top of the theme
      // palette's base — clamped so an extreme slider value
      // can't push alpha outside the valid [0, 1] range the
      // shader (and the browser's blend func) expect.
      let opacity = (palette.waves_opacity * state.waves_config.opacity).clamp(0.0, 1.0);
      gl.uniform1f(Some(u), opacity);
    }
    if let Some(ref u) = fx.uniforms.u_horizon_color {
      let c = palette.waves_horizon;
      gl.uniform3f(Some(u), c[0], c[1], c[2]);
    }
    if let Some(ref u) = fx.uniforms.u_wave_color {
      let c = palette.waves_body;
      gl.uniform3f(Some(u), c[0], c[1], c[2]);
    }
    if let Some(ref u) = fx.uniforms.u_crest_color {
      let c = palette.waves_crest;
      gl.uniform3f(Some(u), c[0], c[1], c[2]);
    }
    if let Some(ref u) = fx.uniforms.u_wave_scale {
      gl.uniform1f(Some(u), state.waves_config.scale);
    }
    if let Some(ref u) = fx.uniforms.u_wave_ratio {
      gl.uniform1f(Some(u), state.waves_config.ratio);
    }
    if let Some(ref u) = fx.uniforms.u_wave_speed {
      gl.uniform1f(Some(u), state.waves_config.speed);
    }
    if let Some(ref u) = fx.uniforms.u_wave_swell {
      gl.uniform1f(Some(u), state.waves_config.swell);
    }
    if let Some(ref u) = fx.uniforms.u_wave_turbulence {
      gl.uniform1f(Some(u), state.waves_config.turbulence);
    }
    if let Some(ref u) = fx.uniforms.u_wave_tilt {
      gl.uniform1f(Some(u), state.waves_config.tilt);
    }
    if let Some(ref u) = fx.uniforms.u_wave_zoom {
      gl.uniform1f(Some(u), state.waves_config.zoom);
    }
    if let Some(ref u) = fx.uniforms.u_wave_horizon_height {
      gl.uniform1f(Some(u), state.waves_config.horizon_height);
    }
    if let Some(ref u) = fx.uniforms.u_wave_fog_depth {
      gl.uniform1f(Some(u), state.waves_config.fog_depth);
    }
    if let Some(ref u) = fx.uniforms.u_wave_brightness {
      gl.uniform1f(Some(u), state.waves_config.brightness);
    }
    gl.draw_arrays(GL::TRIANGLES, 0, 3);
  }

  // 2. Light Rays
  if draw_rays && let Some(ref fx) = state.rays {
    gl.use_program(Some(&fx.program));
    gl.bind_vertex_array(Some(&fx.vao));
    if let Some(ref u) = fx.uniforms.i_resolution {
      gl.uniform2f(Some(u), w, h);
    }
    if let Some(ref u) = fx.uniforms.i_time {
      gl.uniform1f(Some(u), t);
    }
    if let Some(ref u) = fx.uniforms.u_mouse {
      gl.uniform2f(Some(u), mx, my);
    }
    if let Some(ref u) = fx.uniforms.u_opacity {
      gl.uniform1f(Some(u), palette.rays_opacity);
    }
    if let Some(ref u) = fx.uniforms.u_rays_color {
      let c = palette.rays_color;
      gl.uniform3f(Some(u), c[0], c[1], c[2]);
    }
    gl.draw_arrays(GL::TRIANGLES, 0, 3);
  }

  // 3. Particles — uses CSS-pixel coordinates, so pass the CSS
  // viewport size (not the drawing-buffer size) as iResolution.
  if draw_particles && let Some((ref fx, count)) = state.particles {
    let css_w = web_sys::window()
      .and_then(|win| win.inner_width().ok())
      .and_then(|v| v.as_f64())
      .unwrap_or(800.0) as f32;
    let css_h = web_sys::window()
      .and_then(|win| win.inner_height().ok())
      .and_then(|v| v.as_f64())
      .unwrap_or(600.0) as f32;

    gl.use_program(Some(&fx.program));
    gl.bind_vertex_array(Some(&fx.vao));
    if let Some(ref u) = fx.uniforms.i_resolution {
      gl.uniform2f(Some(u), css_w, css_h);
    }
    if let Some(ref u) = fx.uniforms.i_time {
      gl.uniform1f(Some(u), t);
    }
    if let Some(ref u) = fx.uniforms.u_dpr {
      // `w` is the drawing-buffer (device-pixel) width; dividing
      // by the CSS width recovers the DPR without a second
      // `window.devicePixelRatio()` read. See the `uDpr` comment
      // in `VERT_PARTICLES` for why this is required at all.
      let dpr = if css_w > 0.0 { w / css_w } else { 1.0 };
      gl.uniform1f(Some(u), dpr);
    }
    if let Some(ref u) = fx.uniforms.u_color {
      let c = palette.particles_color;
      gl.uniform3f(Some(u), c[0], c[1], c[2]);
    }
    if let Some(ref u) = fx.uniforms.u_opacity {
      gl.uniform1f(Some(u), palette.particles_opacity);
    }
    gl.draw_arrays(GL::POINTS, 0, count as i32);
  }
}
