use serde_json::error;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AnalyserNode, AudioContext, Blob, BlobEvent, CanvasRenderingContext2d, HtmlCanvasElement,
  MediaRecorder, MediaStream,
};

use crate::{
  model::VisualizeColor,
  utils::{get_user_media, Timer},
};

#[derive(Clone)]
pub struct WaveRecorder {
  recorder: Option<MediaRecorder>,
  canvas: Option<HtmlCanvasElement>,
  visualize_color: VisualizeColor,
  audio_ctx: Option<AudioContext>,
  analyser: Option<AnalyserNode>,
  timer: Rc<Timer>,
  this: Option<Rc<RefCell<Self>>>,
  is_init: Rc<RefCell<bool>>,
}

impl WaveRecorder {
  pub fn new(
    visualize_color: VisualizeColor,
    canvas: Option<HtmlCanvasElement>,
  ) -> Result<Rc<RefCell<Self>>, error::Error> {
    let wave = Rc::new(RefCell::new(WaveRecorder {
      recorder: None,
      canvas,
      visualize_color,
      audio_ctx: None,
      analyser: None,
      this: None,
      timer: Rc::new(Timer::new()),
      is_init: Rc::new(RefCell::new(false)),
    }));
    wave.borrow_mut().this = Some(wave.clone());
    Ok(wave)
  }

  fn get_recorder(&self) -> Result<&MediaRecorder, &str> {
    self.recorder.as_ref().ok_or("Failed to get mediaRecorder")
  }

  fn get_canvas(&self) -> Result<&HtmlCanvasElement, &str> {
    self.canvas.as_ref().ok_or("Failed to get canvas ")
  }

  pub fn set_canvas(&mut self, canvas: Option<HtmlCanvasElement>) {
    self.canvas = canvas;
  }

  pub async fn start(&mut self) -> Result<(), JsValue> {
    let stream = get_user_media(
      Some("{ device_id: 'default',echo_cancellation: true }"),
      None,
    )
    .await?;
    if self.canvas.is_some() {
      let recorder = MediaRecorder::new_with_media_stream(&stream).ok();
      self.recorder = recorder;
      self.get_recorder()?.start()?;
      self.visualize_stream_connect(stream)?;
      self.subscribe();
      self.timer.start();
    }
    Ok(())
  }

  pub async fn stop(&self) -> Result<Blob, JsValue> {
    let stop_promise = js_sys::Promise::new(&mut |resolve, reject| {
      let callback = Closure::wrap(Box::new(move |event: BlobEvent| {
        if let Some(blob) = event.data() {
          let _ = resolve.call1(&JsValue::undefined(), &blob);
        } else {
          let _ = reject.call1(&JsValue::undefined(), &JsValue::from("get blob error"));
        }
      }) as Box<dyn FnMut(_)>);
      let _ = self
        .get_recorder()
        .unwrap()
        .add_event_listener_with_callback("dataavailable", callback.as_ref().unchecked_ref());
      callback.forget();
    });
    self.get_recorder()?.stop()?;
    let js_blob = JsFuture::from(stop_promise).await?;
    let blob = JsCast::unchecked_into::<Blob>(js_blob);
    self.timer.stop();
    Ok(blob)
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) {
    self.visualize_color = visualize_color;
  }

  pub fn visualize_stream_connect(&mut self, stream: MediaStream) -> Result<(), JsValue> {
    self.audio_ctx = AudioContext::new().ok();
    self.analyser = self
      .audio_ctx
      .as_mut()
      .ok_or("Failed to get audio context")?
      .create_analyser()
      .ok();
    self
      .analyser
      .as_mut()
      .map_or((), |analyser| analyser.set_fft_size(256));
    let source = self
      .audio_ctx
      .as_ref()
      .ok_or("Failed to get audio context")?
      .create_media_stream_source(&stream)?;
    let analyser = self.analyser.as_ref().ok_or("Failed to get analyser")?;
    source.connect_with_audio_node(analyser)?;
    Ok(())
  }

  fn visualize(&self) -> Result<(), JsValue> {
    let buffer_length = self
      .analyser
      .as_ref()
      .ok_or("Failed to get buffer length")?
      .frequency_bin_count();
    let mut data_array: Vec<u8> = vec![0; buffer_length.try_into().unwrap()];
    let mut x = 0f64;
    let bar_width = 4f64;
    let canvas_context = self
      .get_canvas()?
      .get_context("2d")?
      .ok_or("Failed to get canvas context")?
      .dyn_into::<CanvasRenderingContext2d>()?;
    self
      .analyser
      .as_ref()
      .ok_or("Failed to get analyser")?
      .get_byte_frequency_data(&mut data_array);
    let canvas_height = self.get_canvas()?.height();
    let canvas_width = self.get_canvas()?.width();
    canvas_context.set_global_alpha(self.visualize_color.opacity);
    canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.background));
    canvas_context.fill_rect(0f64, 0f64, canvas_width as f64, canvas_height as f64);
    canvas_context.begin_path();
    for i in 0..buffer_length {
      let bar_height = (data_array.get(i as usize).unwrap_or(&0) / 4) as f64;
      canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.rect_color));
      canvas_context.round_rect_with_f64(
        x,
        canvas_height as f64 / 2.0 - (bar_height / 2.0),
        bar_width,
        bar_height,
        bar_width / 2.0,
      )?;
      x += bar_width * 2.0 + 1.0;
    }
    canvas_context.fill();
    canvas_context.close_path();
    Ok(())
  }

  fn subscribe(&self) {
    if *self.is_init.borrow() {
      return;
    }
    if let Some(wave) = &self.this {
      let wave = wave.clone();
      self.timer.subscribe(move || {
        let _ = wave.borrow().visualize();
      });
    }
    *self.is_init.borrow_mut() = true;
  }
}
