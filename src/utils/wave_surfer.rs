use js_sys::ArrayBuffer;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  AudioContext, AudioContextOptions, Blob, CanvasRenderingContext2d, File, HtmlCanvasElement,
};

use crate::model::VisualizeColor;

use super::{get_dpr, get_window, read_file};

#[derive(Clone)]
pub struct WaveSurfer {
  canvas: HtmlCanvasElement,
  visualize_color: VisualizeColor,
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
    Ok(WaveSurfer {
      canvas,
      visualize_color,
    })
  }

  pub async fn load_from_array_buffer(&self, array_buffer: ArrayBuffer) -> Result<(), JsValue> {
    let mut options = AudioContextOptions::new();
    options.sample_rate(16000.0);
    let audio_ctx = AudioContext::new_with_context_options(&options)?;
    let decode_promise = audio_ctx.decode_audio_data(&array_buffer)?;
    let decode_buffer = JsFuture::from(decode_promise).await?;
    let decode_buffer = decode_buffer.dyn_into::<web_sys::AudioBuffer>()?;
    let channel_data = decode_buffer.get_channel_data(0)?;
    self.visualize(channel_data)?;
    Ok(())
  }

  pub async fn load_from_blob(&self, blob: Blob) -> Result<(), JsValue> {
    let file = File::new_with_blob_sequence(&JsValue::from(blob), "tmp.wav")?;
    let array_buffer = read_file(file).await?;
    self.load_from_array_buffer(array_buffer).await
  }

  pub async fn load_from_file(&self, file: File) -> Result<(), JsValue> {
    let array_buffer = read_file(file).await?;
    self.load_from_array_buffer(array_buffer).await
  }

  pub async fn start(&self) -> Result<(), JsValue> {
    Ok(())
  }

  pub async fn stop(self) -> Result<(), JsValue> {
    Ok(())
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) {
    self.visualize_color = visualize_color;
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
