use stylist::{self, style};
use yew::{prelude::*, Callback};
use yew_icons::{Icon, IconId};

use crate::{components::Input, utils::style};

#[derive(Properties, PartialEq)]
pub struct Props {
  #[prop_or_default]
  pub value: String,
  #[prop_or_default]
  pub onsearch: Callback<String>,
}

#[function_component]
pub fn Search(props: &Props) -> Html {
  let class_name = get_class_name();
  let onsearch = props.onsearch.clone();
  let onsearch_clone = onsearch.clone();
  let keyword = use_state(|| "".to_string());

  let onenter = {
    let keyword_clone = keyword.clone();
    Callback::from(move |_: ()| {
      onsearch.emit((*keyword_clone).clone());
    })
  };

  let keyword_clone = keyword.clone();
  let onchange = Callback::from(move |val: String| {
    keyword_clone.set(val);
  });

  let keyword_clone = keyword.clone();
  let onclear = Callback::from(move |_| {
    keyword_clone.set("".to_string());
    onsearch_clone.emit("".to_string());
  });

  let class_name = {
    let empty = if keyword.is_empty() {"empty"} else {""};
    format!("{class_name} {empty}")
  };

  html! {
    <div class={class_name}>
      <Input {onchange} value={(*keyword).clone()} {onenter} icon={Some(IconId::BootstrapSearch)} />
      <Icon class="clear" icon_id={IconId::OcticonsXCircleFill16} width="16px" height="16px" onclick={onclear} />
    </div>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      position: relative;
      &:not(&.empty):hover svg {
        opacity: 0;  
        visibility: hidden;
      }
      &:not(&.empty):hover .clear {
        opacity: 1;  
        visibility: visible;  
      }
      .clear {
        cursor: pointer;
        opacity: 0;  
        position: absolute;
        visibility: hidden;  
        transition: all 0.2s ease-in-out;
        inset-inline-end: 9px;
        inset-block-end: 0;
        inset-block-start: 0;
        margin: auto;
        color: white;
      }
    "#
  ))
}
