use std::rc::Rc;
use js_sys::ArrayBuffer;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AudioContext, AudioContextOptions, Blob, CanvasRenderingContext2d, Event, File, HtmlAudioElement,
  HtmlCanvasElement, Url,
};

use crate::model::VisualizeColor;

use super::{array_buffer_to_blob_url, get_dpr, get_window, read_file, Timer, WaveProgress};

pub struct WaveSurfer {
  canvas: HtmlCanvasElement,
  audio: HtmlAudioElement,
  progress: Rc<WaveProgress>,
  visualize_color: VisualizeColor,
  is_initial: Option<bool>,
  timer: Rc<Timer>,
}

impl WaveSurfer {
  pub fn new(container: String, visualize_color: VisualizeColor) -> Result<Self, JsValue> {
    let document = get_window().document().ok_or("error")?;
    let canvas = document
      .create_element("canvas")?
      .dyn_into::<HtmlCanvasElement>()?;
    let container_dom = document.query_selector(&container)?.ok_or("error")?;
    let height = container_dom.client_height();
    let width = container_dom.client_width();
    let dpr = get_dpr() as i32;
    canvas.set_height((height * dpr) as u32);
    canvas.set_width((width * dpr) as u32);
    container_dom.append_child(&canvas)?;
    let audio = document
      .create_element("audio")?
      .dyn_into::<HtmlAudioElement>()?;
    let progress = WaveProgress::new(container_dom, visualize_color.clone())?;
    Ok(WaveSurfer {
      canvas,
      audio,
      progress: Rc::new(progress),
      visualize_color,
      is_initial: None,
      timer: Rc::new(Timer::new()),
    })
  }

  pub async fn load_from_array_buffer(&mut self, array_buffer: ArrayBuffer) -> Result<(), JsValue> {
    let mut options = AudioContextOptions::new();
    options.sample_rate(16000.0);
    self.subscribe();
    self.bind_event()?;
    self.set_src(&array_buffer)?;
    let audio_ctx = AudioContext::new_with_context_options(&options)?;
    let decode_promise = audio_ctx.decode_audio_data(&array_buffer)?;
    let decode_buffer = JsFuture::from(decode_promise).await?;
    let decode_buffer = decode_buffer.dyn_into::<web_sys::AudioBuffer>()?;
    let channel_data = decode_buffer.get_channel_data(0)?;
    self.visualize(channel_data)?;
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

  pub fn start(&self) -> Result<(), JsValue> {
    self.audio.play();
    Ok(())
  }

  pub fn duration(&self) -> f64 {
    self.audio.duration()
  }

  pub fn stop(self) -> Result<(), JsValue> {
    self.audio.pause();
    Ok(())
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) {
    self.visualize_color = visualize_color;
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
      let _ = progress.update_progress(format!("{}%", width));
    });
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
      let _ = progress.update_progress(format!("{}%", width));
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

  fn visualize(&self, channel_data: Vec<f32>) -> Result<(), JsValue> {
    let length = channel_data.len();
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
    channel_data.iter().enumerate().for_each(|(i, item)| {
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
