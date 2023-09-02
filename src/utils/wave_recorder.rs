use std::{cell::RefCell, rc::Rc};
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Array;
use serde::Serialize;
use serde_json::error;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AnalyserNode, AudioContext, Blob, BlobEvent, BlobPropertyBag, CanvasRenderingContext2d,
  HtmlCanvasElement, MediaRecorder, MediaStream, MediaStreamConstraints,
};

use super::{get_window, request_animation_frame};

#[derive(Serialize)]
pub struct Constraints {
  device_id: String,
  echo_cancellation: bool,
}

#[derive(Clone)]
pub struct VisualizeColor {
  pub background: String,
  pub rect_color: String,
  pub opacity: f64,
}

#[derive(Clone)]
pub struct WaveRecorder {
  pub chunks: Rc<RefCell<Vec<Blob>>>,
  recorder: Option<MediaRecorder>,
  canvas: Option<HtmlCanvasElement>,
  visualize_color: VisualizeColor,
  constraints: MediaStreamConstraints,
  audio_ctx: Option<AudioContext>,
  analyser: Option<AnalyserNode>,
  running: Rc<RefCell<bool>>,
}

impl WaveRecorder {
  pub fn new(
    visualize_color: VisualizeColor,
    canvas: Option<HtmlCanvasElement>,
  ) -> Result<Self, error::Error> {
    let mut constraints = MediaStreamConstraints::new();
    let audio_constraints = JsValue::from_serde(&Constraints {
      device_id: "default".to_string(),
      echo_cancellation: true,
    })?;
    constraints.audio(&audio_constraints);
    Ok(WaveRecorder {
      chunks: Rc::new(RefCell::new(vec![])),
      recorder: None,
      canvas,
      visualize_color,
      constraints,
      audio_ctx: None,
      analyser: None,
      running: Rc::new(RefCell::new(false)),
    })
  }
  pub async fn get_media(&self) -> Result<MediaStream, JsValue> {
    let window = get_window();
    let promise = window
      .navigator()
      .media_devices()?
      .get_user_media_with_constraints(&self.constraints)?;
    let result = JsFuture::from(promise).await?;
    Ok(result.into())
  }

  fn get_recorder(&self) -> Result<&MediaRecorder, &str> {
    self.recorder.as_ref().ok_or("Failed to get mediaRecorder")
  }

  fn get_canvas(&self) -> Result<&HtmlCanvasElement, &str> {
    self.canvas.as_ref().ok_or("Failed to get canvas ")
  }

  pub fn set_canvas(&mut self, canvas: Option<HtmlCanvasElement>) -> () {
    self.canvas = canvas;
  }

  pub async fn start(&mut self) -> Result<(), JsValue> {
    let stream = self.get_media().await?;
    if let Some(_) = &self.canvas {
      let recorder = MediaRecorder::new_with_media_stream(&stream).ok();
      self.recorder = recorder;
      self.bind_event()?;
      self.get_recorder()?.start_with_time_slice(100)?;
      *self.running.borrow_mut() = true;
      self.visualize_stream_connect(stream)?;
      self.animate_drawing();
    }
    Ok(())
  }

  pub fn bind_event(&mut self) -> Result<(), JsValue> {
    let chunks = self.chunks.clone();
    let callback = Closure::wrap(Box::new(move |event: BlobEvent| {
      if let Some(blob) = event.data() {
        chunks.borrow_mut().push(blob);
      }
    }) as Box<dyn FnMut(_)>);
    self
      .get_recorder()?
      .add_event_listener_with_callback("dataavailable", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
  }

  pub async fn stop(&mut self) -> Result<Blob, JsValue> {
    let stop_promise = js_sys::Promise::new(&mut |resolve, reject| {
      let chunks = self.chunks.borrow();
      let blob_parts = Array::new_with_length(chunks.len() as u32);
      for (i, blob) in chunks.iter().enumerate() {
        blob_parts.set(i as u32, blob.clone().into());
      }
      let mut options = BlobPropertyBag::new();
      options.type_("audio/ogg; codecs=opus");
      Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options).map_or_else(
        |err| {
          let _ = reject.call1(&JsValue::undefined(), &err);
        },
        |blob| {
          let _ = resolve.call1(&JsValue::undefined(), &blob);
        },
      );
    });
    self.get_recorder()?.stop()?;
    *self.running.borrow_mut() = false;
    self.chunks.borrow_mut().clear();
    let js_blob = JsFuture::from(stop_promise).await?;
    let blob = wasm_bindgen::JsCast::unchecked_into::<Blob>(js_blob);
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
    source.connect_with_audio_node(&analyser)?;
    Ok(())
  }

  pub fn draw_rounded_rect(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
  ) -> Result<(), JsValue> {
    ctx.begin_path();
    ctx.move_to(x + radius, y);
    ctx.arc_to(x + width, y, x + width, y + height, radius)?;
    ctx.arc_to(x + width, y + height, x, y + height, radius)?;
    ctx.arc_to(x, y + height, x, y, radius)?;
    ctx.arc_to(x, y, x + width, y, radius)?;
    ctx.close_path();
    ctx.fill();
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
    for i in 0..buffer_length {
      let bar_height = (data_array.get(i as usize).unwrap_or(&0) / 4) as f64;
      canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.rect_color));
      WaveRecorder::draw_rounded_rect(
        &canvas_context,
        x.into(),
        canvas_height as f64 / 2.0 - (bar_height / 2.0),
        bar_width.into(),
        bar_height,
        bar_width as f64 / 2.0,
      )?;
      x += bar_width * 2.0 + 1.0;
    }
    Ok(())
  }

  pub fn animate_drawing(&self) -> () {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    let wave = self.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
      let _ = wave.visualize();
      if *wave.running.borrow() {
        request_animation_frame(f.borrow().as_ref().unwrap());
      }
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
  }
}
