use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlElement, Element};

use crate::model::VisualizeColor;

use super::get_window;

fn create_element(css_text: &str) -> Result<HtmlElement, JsValue> {
  let document = get_window().document().ok_or("error")?;
  let element = document.create_element("div")?.dyn_into::<HtmlElement>()?;
  let css_text = format!(
    "pointer-events:none; position: absolute; height: 100%; top: 0; left: 0;{}",
    css_text
  );
  element.style().set_css_text(&css_text);
  Ok(element)
}
#[derive(Clone)]
pub struct WaveProgress {
  visualize_color: VisualizeColor,
  progress: HtmlElement,
  cursor: HtmlElement,
}

impl WaveProgress {
  pub fn new(container: Element, visualize_color: VisualizeColor) -> Result<Self, JsValue> {
    let progress = create_element(&format!(
      "opacity: 0.1; background: {};",
      visualize_color.background
    ))?;
    let cursor = create_element(&format!(
      "width: 2px; border-radius: 2px; background: {};",
      visualize_color.rect_color
    ))?;
    container.append_child(&progress)?;
    container.append_child(&cursor)?;
    Ok(WaveProgress {
      visualize_color,
      progress,
      cursor,
    })
  }

  pub fn update_progress(&self, value: String) -> Result<(), JsValue> {
    self.progress.style().set_property("left", &value)?;
    self.cursor.style().set_property("left", &value)?;
    Ok(())
  }
}
