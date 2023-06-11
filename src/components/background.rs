use std::rc::Rc;
use stylist::{self, style};
use yew::{function_component, html, use_effect_with_deps, use_node_ref, Html};

use crate::utils::{ribbon::{ColorSet, Position, Ribbons}, style};

#[function_component]
pub fn Background() -> Html {
  let class_name = get_class_name();
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
    <div class={class_name}>
      <canvas ref={canvas_ref.as_ref()} />
      <div class="mask" />
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(
    style!(
      // A CSS string literal
      r#"
        block-size: 100%;
        inline-size: 100%;
        position: fixed;
        z-index: -1;
        inset-block: 0;
        
        .mask {
          block-size: 100%;
          inline-size: 100%;
          position: absolute;
          inset-block: 0;
          backdrop-filter: blur(10px);
        }

        canvas {
          background: #161c20;
          block-size: 100%;
          inline-size: 100%;
        }
    "#
    )
  )
}
