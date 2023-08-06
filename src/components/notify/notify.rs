
use stylist::{self, style};
use yew::prelude::*;

use super::NoticeContext;
use crate::utils::style;



#[function_component]
pub fn Notify() -> Html {
  let class_name = get_class_name();
  let notice_list = use_context::<NoticeContext>().map_or(vec![], |x| x.0.clone());

  html! {
    <div class={class_name} >
    {{ notice_list.len()}}
      { for notice_list.iter().map(|item|{
        html!{
          <div>{item.content.clone()}</div>
      }})}
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        position: fixed;
        inline-size: 400px;
        block-size: 100px;
        inset-block-start: 20px;
        inset-inline: 0;
        padding: 20px;
        margin: auto;
        color: var(--font-color);
        background: rgba(var(--theme-color-rgb), 0.8);
        border-radius: calc(var(--radius) / 3);
        backdrop-filter: blur(10px);
    "#
  ))
}
