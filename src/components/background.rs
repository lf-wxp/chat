use std::rc::Rc;

use stylist::{self, style};
use yew::{function_component, html, use_effect_with_deps, use_node_ref, Html};

use crate::utils::ribbon::{ColorSet, Position, Ribbons};


#[function_component]
pub fn Background() -> Html {
  let class_name = get_style().unwrap_or_default();
  let canvas_ref = Rc::new(use_node_ref());
  let canvas_ref_clone = canvas_ref.clone();

  use_effect_with_deps(
    move |_| {
      let ribbon_background = Ribbons {
        canvas: canvas_ref_clone,
        color_set: ColorSet {
          saturation: "60%".to_owned(),
          brightness: "50%".to_owned(),
          alpha: 0.5,
          cycle_speed: 9.0,
        },
        vertical_position: Position::Random,
        horizontal_speed: 100.0,
        ribbon_count: 3,
        stroke_size: 0.0,
        parallax_amount: -0.5,
        animate_sections: false,
        ribbons: vec![],
        scroll: 0.0,
      };
      ribbon_background.init();
    },
    (),
  );

  html! {
    <canvas ref={canvas_ref.as_ref()} class={class_name} />
  }
}

fn get_style() -> Result<String, stylist::Error> {
  Ok(
    style!(
      // A CSS string literal
      r#"
      width: 100vw;
      height: 100vh;
      position: absolute;
      left: 0;
      top: 0;
    "#
    )?
    .get_class_name()
    .to_owned(),
  )
}
