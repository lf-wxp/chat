use js_sys::{ArrayBuffer, Promise};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AudioContext, AudioContextOptions, Blob, CanvasRenderingContext2d, Element, Event, File,
  HtmlAudioElement, HtmlCanvasElement, PointerEvent, Url,
};

use crate::{
  model::VisualizeColor,
  utils::{array_buffer_to_blob_url, get_dpr, get_document, read_file, Timer, WaveProgress},
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum WaveEvent {
  PlayState,
}

pub enum WaveEventCallback {
  PlayStateCallback(Rc<dyn Fn(bool)>),
}

pub struct WaveSurfer {
  canvas: HtmlCanvasElement,
  audio: HtmlAudioElement,
  progress: Rc<RefCell<WaveProgress>>,
  visualize_color: VisualizeColor,
  is_initial: Option<bool>,
  timer: Rc<Timer>,
  duration: f64,
  handlers: Rc<RefCell<HashMap<WaveEvent, Vec<WaveEventCallback>>>>,
  channel_data: Vec<f32>,
}

impl WaveSurfer {
  pub fn new(container: Element, visualize_color: VisualizeColor) -> Result<Self, JsValue> {
    let document = get_document();
    let canvas = document
      .create_element("canvas")?
      .dyn_into::<HtmlCanvasElement>()?;
    let height = container.client_height();
    let width = container.client_width();
    let dpr = get_dpr() as i32;
    canvas.set_height((height * dpr) as u32);
    canvas.set_width((width * dpr) as u32);
    canvas
      .style()
      .set_css_text("inline-size:100%; block-size:100%");
    container.append_child(&canvas)?;
    let audio = document
      .create_element("audio")?
      .dyn_into::<HtmlAudioElement>()?;
    let progress = WaveProgress::new(container, visualize_color.clone())?;
    Ok(WaveSurfer {
      canvas,
      audio,
      progress: Rc::new(RefCell::new(progress)),
      visualize_color,
      is_initial: None,
      timer: Rc::new(Timer::new()),
      duration: 0f64,
      handlers: Rc::new(RefCell::new(HashMap::new())),
      channel_data: vec![],
    })
  }

  pub async fn load_from_array_buffer(&mut self, array_buffer: ArrayBuffer) -> Result<(), JsValue> {
    let mut options = AudioContextOptions::new();
    options.sample_rate(16000.0);
    self.set_src(&array_buffer)?;
    let audio_ctx = AudioContext::new_with_context_options(&options)?;
    let decode_promise = audio_ctx.decode_audio_data(&array_buffer)?;
    let decode_buffer = JsFuture::from(decode_promise).await?;
    let decode_buffer = decode_buffer.dyn_into::<web_sys::AudioBuffer>()?;
    let channel_data = decode_buffer.get_channel_data(0)?;
    self.duration = decode_buffer.duration();
    self.channel_data = channel_data;
    self.visualize()?;
    self.subscribe();
    self.bind_event()?;
    self.bind_seek()?;
    self.bind_play_state()?;
    Ok(())
  }

  pub async fn load_from_blob(&mut self, blob: Blob) -> Result<(), JsValue> {
    let file = File::new_with_blob_sequence(&JsValue::from(&blob), "tmp.wav")?;
    let array_buffer = read_file(file).await?;
    self.load_from_array_buffer(array_buffer).await
  }

  pub async fn load_from_file(&mut self, file: File) -> Result<(), JsValue> {
    let array_buffer = read_file(file).await?;
    self.load_from_array_buffer(array_buffer).await
  }

  pub fn start(&self) -> Result<Promise, JsValue> {
    self.audio.play()
  }

  pub fn get_duration(&self) -> f64 {
    self.duration
  }

  pub fn stop(&self) -> Result<(), JsValue> {
    self.audio.pause()
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) -> Result<(), JsValue> {
    self.visualize_color = visualize_color.clone();
    self.progress.borrow_mut().set_color(visualize_color)?;
    self.visualize()
  }

  pub fn on(&mut self, event_type: WaveEvent, handler: WaveEventCallback) {
    let mut handlers = self.handlers.borrow_mut();
    let entry = handlers.entry(event_type).or_insert_with(Vec::new);
    entry.push(handler);
  }

  fn set_src(&self, array_buffer: &ArrayBuffer) -> Result<(), JsValue> {
    let prev_url = self.audio.src();
    Url::revoke_object_url(&prev_url)?;
    let url = array_buffer_to_blob_url(array_buffer, "")?;
    self.audio.set_src(&url);
    Ok(())
  }

  fn subscribe(&self) {
    if self.is_initial.is_some() {
      return;
    }
    let audio = self.audio.clone();
    let progress = self.progress.clone();
    self.timer.subscribe(move || {
      let time = audio.current_time();
      let width = time / audio.duration() * 100.0;
      let _ = progress.borrow_mut().update_progress(format!("{}%", width));
    });
  }

  fn bind_play_state(&self) -> Result<(), JsValue> {
    let audio = self.audio.clone();
    let handlers = self.handlers.clone();
    let sate_callback = Closure::wrap(Box::new(move |_: Event| {
      if let Some(handlers) = handlers.borrow().get(&WaveEvent::PlayState) {
        handlers.iter().for_each(|handler| {
          if let WaveEventCallback::PlayStateCallback(handler) = handler {
            handler(!(audio.ended() || audio.paused()));
          }
        })
      }
    }) as Box<dyn FnMut(_)>);
    self
      .audio
      .add_event_listener_with_callback("play", sate_callback.as_ref().unchecked_ref())?;
    self
      .audio
      .add_event_listener_with_callback("pause", sate_callback.as_ref().unchecked_ref())?;
    self
      .audio
      .add_event_listener_with_callback("end", sate_callback.as_ref().unchecked_ref())?;
    sate_callback.forget();
    Ok(())
  }

  fn bind_seek(&self) -> Result<(), JsValue> {
    let audio = self.audio.clone();
    let duration = self.duration;
    let click_callback = Closure::wrap(Box::new(move |event: PointerEvent| {
      if let Some(canvas) = event
        .current_target()
        .and_then(|x| x.dyn_into::<HtmlCanvasElement>().ok())
      {
        let rate = event.offset_x() as f64 / canvas.client_width() as f64;
        audio.set_current_time(duration * rate);
      }
    }) as Box<dyn FnMut(_)>);
    self
      .canvas
      .add_event_listener_with_callback("click", click_callback.as_ref().unchecked_ref())?;
    click_callback.forget();
    Ok(())
  }

  fn bind_event(&mut self) -> Result<(), JsValue> {
    if self.is_initial.is_some() {
      return Ok(());
    };
    let audio_clone = self.audio.clone();
    let progress = self.progress.clone();
    let timeupdate_callback = Closure::wrap(Box::new(move |_: Event| {
      let time = audio_clone.current_time();
      let width = time / audio_clone.duration() * 100.0;
      let _ = progress.borrow_mut().update_progress(format!("{}%", width));
    }) as Box<dyn FnMut(_)>);

    self.is_initial = Some(true);
    self.audio.add_event_listener_with_callback(
      "timeupdate",
      timeupdate_callback.as_ref().unchecked_ref(),
    )?;
    timeupdate_callback.forget();

    let timer = self.timer.clone();
    let play_callback = Closure::wrap(Box::new(move |_: Event| {
      timer.as_ref().start();
    }) as Box<dyn FnMut(_)>);
    self
      .audio
      .add_event_listener_with_callback("play", play_callback.as_ref().unchecked_ref())?;
    play_callback.forget();

    let timer = self.timer.clone();
    let end_callback = Closure::wrap(Box::new(move |_: Event| {
      timer.as_ref().stop();
    }) as Box<dyn FnMut(_)>);
    self
      .audio
      .add_event_listener_with_callback("pause", end_callback.as_ref().unchecked_ref())?;
    self
      .audio
      .add_event_listener_with_callback("emptied", end_callback.as_ref().unchecked_ref())?;
    end_callback.forget();

    Ok(())
  }

  fn visualize(&self) -> Result<(), JsValue> {
    let length = self.channel_data.len();
    let height = self.canvas.height() as f64;
    let width = self.canvas.width() as f64;
    let half_height = height / 2f64;
    let pixel_ratio = get_dpr();

    let bar_width = 2.0 * pixel_ratio;
    let bar_gap = bar_width / 2.0;
    let bar_radius = 2f64;
    let bar_index_scale = width / (bar_width + bar_gap) / (length as f64);
    let v_scale = 1f64;
    let bar_align = "";

    let mut prev_x = 0f64;
    let mut max_top = 0f64;
    let mut max_bottom = 0f64;
    let canvas_context = self
      .canvas
      .get_context("2d")?
      .ok_or("Failed to get canvas context")?
      .dyn_into::<CanvasRenderingContext2d>()?;
    canvas_context.set_global_alpha(self.visualize_color.opacity);
    canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.background));
    canvas_context.fill_rect(0f64, 0f64, width, height);
    canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.rect_color));
    self.channel_data.iter().enumerate().for_each(|(i, item)| {
      let x = (i as f64 * bar_index_scale).round();
      if x > prev_x {
        let top_bar_height = (max_top * half_height * v_scale).round();
        let bottom_bar_height = (max_bottom * half_height * v_scale).round();
        let bar_height = top_bar_height + bottom_bar_height;

        // Vertical alignment
        let mut y = half_height - top_bar_height;
        if bar_align == "top" {
          y = 0f64;
        } else if bar_align == "bottom" {
          y = height - bar_height
        }
        let _ = canvas_context.round_rect_with_f64(
          prev_x * (bar_width + bar_gap),
          y,
          bar_width,
          bar_height,
          bar_radius,
        );
        prev_x = x;
        max_top = 0f64;
        max_bottom = 0f64;
      }

      let magnitude_top = item.abs() as f64;
      let magnitude_bottom = item.abs() as f64;
      if magnitude_top > max_top {
        max_top = magnitude_top;
      }
      if magnitude_bottom > max_bottom {
        max_bottom = magnitude_bottom;
      }
    });
    canvas_context.fill();
    Ok(())
  }
}
