use std::{cell::RefCell, rc::Rc};

use gloo_utils::format::JsValueSerdeExt;
use js_sys::Array;
use serde::Serialize;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{
  AnalyserNode, AudioContext, Blob, BlobEvent, BlobPropertyBag,
  CanvasRenderingContext2d, HtmlCanvasElement, MediaRecorder, MediaStream, MediaStreamConstraints,
};

use super::{get_window, request_animation_frame};

#[derive(Serialize)]
struct Constraints {
  device_id: String,
  echo_cancellation: bool,
}

struct VisualizeColor {
  background: String,
  rect_color: String,
  opacity: f64,
}
struct WaveRecorder {
  chunks: Rc<RefCell<Vec<Blob>>>,
  recorder: Option<MediaRecorder>,
  canvas: Option<HtmlCanvasElement>,
  visualize_color: VisualizeColor,
  constraints: MediaStreamConstraints,
  audio_ctx: Option<AudioContext>,
  analyser: Option<AnalyserNode>,
  animation_handler: Option<u8>,
}

impl WaveRecorder {
  pub fn new(visualize_color: VisualizeColor, canvas: Option<HtmlCanvasElement>) -> Self {
    let mut constraints = MediaStreamConstraints::new();
    let audio_constraints = JsValue::from_serde(&Constraints {
      device_id: "default".to_string(),
      echo_cancellation: true,
    })
    .unwrap();
    constraints.audio(&audio_constraints);
    WaveRecorder {
      chunks: Rc::new(RefCell::new(vec![])),
      recorder: None,
      canvas,
      visualize_color,
      constraints,
      audio_ctx: None,
      analyser: None,
      animation_handler: Some(0),
    }
  }
  pub async fn get_media(&self) -> Result<MediaStream, JsValue> {
    let window = get_window();
    let promise = window
      .navigator()
      .media_devices()?
      .get_user_media_with_constraints(&self.constraints)?;
    let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(result.into())
  }

  pub async fn start(&mut self) -> Result<(), JsValue> {
    let stream = self.get_media().await?;
    if let Some(canvas) = &self.canvas {
      let recorder = MediaRecorder::new_with_media_stream(&stream).ok();
      self.recorder = recorder;
      self.bind_event();
      self.recorder.as_ref().map_or((), |recorder| {
        recorder.start();
      });
    }
    Ok(())
  }

  pub fn bind_event(&mut self) -> Result<(), JsValue> {
    if let Some(recorder) = &self.recorder {
      let chunks = self.chunks.clone();
      let callback = Closure::wrap(Box::new(move |event: BlobEvent| {
        if let Some(blob) = event.data() {
          chunks.borrow_mut().push(blob);
        }
      }) as Box<dyn FnMut(_)>);
      recorder
        .add_event_listener_with_callback("dataavailable", callback.as_ref().unchecked_ref())?;
      callback.forget();
    }
    Ok(())
  }

  pub async fn stop(&mut self) -> Result<(), JsValue> {
    if let Some(recorder) = &self.recorder {
      let stop_promise = js_sys::Promise::new(&mut |resolve, reject| {
        let chunks = self.chunks.borrow();
        let blob_parts = Array::new_with_length(chunks.len() as u32);
        for (i, blob) in chunks.iter().enumerate() {
          blob_parts.set(i as u32, blob.clone().into());
        }
        let mut options = BlobPropertyBag::new();
        options.type_("audio/ogg; codecs=opus");
        match Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options) {
          Ok(blob) => resolve.call0(&blob),
          Err(err) => reject.call0(&err),
        };
      });
      recorder.stop();
      wasm_bindgen_futures::JsFuture::from(stop_promise).await?;
    }
    Ok(())
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) {
    self.visualize_color = visualize_color;
  }

  pub fn visualize_stream_connect(&mut self, stream: MediaStream) -> Result<(), JsValue> {
    if let Some(canvas) = &self.canvas {
      self.audio_ctx = AudioContext::new().ok();
      self.analyser = self.audio_ctx.as_mut().unwrap().create_analyser().ok();
      self
        .analyser
        .as_mut()
        .map_or((), |analyser| analyser.set_fft_size(256));
      let source = self
        .audio_ctx
        .as_ref()
        .unwrap()
        .create_media_stream_source(&stream)?;
      source.connect_with_audio_node(self.analyser.as_ref().unwrap());
    }
    Ok(())
  }

  pub fn draw_rounded_rect(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
  ) {
    ctx.begin_path();
    ctx.move_to(x + radius, y);
    ctx.arc_to(x + width, y, x + width, y + height, radius);
    ctx.arc_to(x + width, y + height, x, y + height, radius);
    ctx.arc_to(x, y + height, x, y, radius);
    ctx.arc_to(x, y, x + width, y, radius);
    ctx.close_path();
    ctx.fill();
  }

  fn visualize(&self) -> Result<(), JsValue> {
    let buffer_length = self.analyser.as_ref().unwrap().frequency_bin_count();
    let mut data_array: Vec<u8> = vec![];
    let bar_width = 2f64;
    let mut x = 0f64;
    let canvas_context = self
      .canvas
      .as_ref()
      .unwrap()
      .get_context("2d")?
      .ok_or_else(|| JsValue::from_str("Failed to get canvas context"))?
      .dyn_into::<CanvasRenderingContext2d>()?;
    self
      .analyser
      .as_ref()
      .unwrap()
      .get_byte_frequency_data(&mut data_array);
    let canvas_height = self.canvas.as_ref().unwrap().height();
    let canvas_width = self.canvas.as_ref().unwrap().width();
    canvas_context.set_global_alpha(self.visualize_color.opacity);
    canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.background));
    canvas_context.fill_rect(
      0f64,
      0f64,
      canvas_width as f64,
      canvas_height as f64,
    );

    (0..buffer_length).into_iter().for_each(|mut x| {
      let bar_height = (data_array.get(x as usize).unwrap() / 4) as f64;
      canvas_context.set_fill_style(&JsValue::from_str(&self.visualize_color.rect_color));
      WaveRecorder::draw_rounded_rect(
        &canvas_context,
        x.into(),
        canvas_height as f64 / 2.0 - (bar_height / 2.0),
        bar_width.into(),
        bar_height,
        bar_width as f64 / 2.0,
      );
      x += (bar_width as u32) * 2 + 1;
    });
    Ok(())
  }

  pub fn animate_drawing(mut self) -> () {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
      self.visualize();
      request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
  }
}
