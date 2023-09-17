use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, HtmlElement};

use crate::model::VisualizeColor;

use super::get_window;

const BASE_CSS_TEXT: &str =
  "pointer-events:none; position: absolute; height: 100%; top: 0; left: 0;";

fn create_element(css_text: &str) -> Result<HtmlElement, JsValue> {
  let document = get_window().document().ok_or("error")?;
  let element = document.create_element("div")?.dyn_into::<HtmlElement>()?;
  let css_text = format!("{BASE_CSS_TEXT}{}", css_text);
  element.style().set_css_text(&css_text);
  Ok(element)
}
pub struct WaveProgress {
  visualize_color: VisualizeColor,
  progress: HtmlElement,
  cursor: HtmlElement,
}

impl WaveProgress {
  pub fn new(container: Element, visualize_color: VisualizeColor) -> Result<Self, JsValue> {
    let progress = create_element(&format!(
      "opacity: 0.5; background: {};",
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

  fn update_color(&self) -> Result<(), JsValue> {
    self
      .progress
      .style()
      .set_property("background", &self.visualize_color.background)?;
    self
      .cursor
      .style()
      .set_property("background", &self.visualize_color.rect_color)?;
    Ok(())
  }

  pub fn update_progress(&self, value: String) -> Result<(), JsValue> {
    self.progress.style().set_property("width", &value)?;
    self.cursor.style().set_property("left", &value)?;
    Ok(())
  }

  pub fn set_color(&mut self, visualize_color: VisualizeColor) -> Result<(), JsValue> {
    self.visualize_color = visualize_color;
    self.update_color()
  }
}
