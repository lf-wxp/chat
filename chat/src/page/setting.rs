use stylist::{self, style};
use yew::prelude::*;

use crate::{
  components::{ThemeSwitch, WaveTest},
  utils::style,
};

#[function_component]
pub fn Setting() -> Html {
  let class_name = get_class_name();

  html! {
    <section class={class_name}>
    <WaveTest />
    <ThemeSwitch />
    </section>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
    "#
  ))
}
