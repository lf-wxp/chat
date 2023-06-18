use bounce::{use_atom, use_atom_value};
use gloo_console::log;
use stylist::{self, style};
use web_sys::HtmlInputElement;
use web_sys::MouseEvent;
use yew::prelude::*;
use yew::Callback;
use yew_icons::{Icon, IconId};

use crate::store::Volume;
use crate::utils::style;

#[function_component]
pub fn VolumeSet() -> Html {
  let slide_node_ref = use_node_ref();
  let volume_handle = use_atom::<Volume>();
  let volume_value = use_atom_value::<Volume>();
  let class_name = get_class_name();
  let indicator_style = format!("inset-block-start: {}%", volume_value.value);
  let slide_style = format!("block-size: {}%", volume_value.value);

  let onclick = {
    let slide = slide_node_ref.clone();
    let volume_handle = volume_handle.clone();
    Callback::from(move |e: MouseEvent| {
      let span = slide.cast::<HtmlInputElement>();
      if let Some(span) = span {
        let val = e.offset_y() as f64 / span.client_height() as f64;
        let volume = Volume {
          value: (val * 100.0) as u8,
          mute: false,
        };
        log!("the event is ", val * 100.0);
        volume_handle.set(volume);
      }
    })
  };

  let toggle_mute = {
    let volume_value = volume_value.clone();
    let volume_handle = volume_handle.clone();
    Callback::from(move |_| {
      let volume = Volume {
        value: volume_value.value,
        mute: !volume_value.mute,
      };
      volume_handle.set(volume);
    })
  };

  let icon_id = if volume_value.mute { IconId::FontAwesomeSolidVolumeXmark} else { IconId::FontAwesomeSolidVolumeHigh };

  html! {
    <section class={class_name}>
      <div class={"slide-box"}>
        <span ref={slide_node_ref} class={"slide-bg"} onclick={onclick} >
          <span class={"slide"} style={slide_style} />
          <span class={"indicator"} style={indicator_style} />
        </span>
      </div>
      <Icon {icon_id} width={"16px"} height={"16px"} onclick={toggle_mute} />
    </section>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
        inline-size: 30px;
        block-size: 30px;
        display: flex;
        justify-content: center;
        align-items: center;
        transition: all 0.2s ease;
        border-end-end-radius: 30%;
        border-end-start-radius: 30%;
        cursor: pointer;
        position: relative;
        color: #8896a4;

        :hover {
          background: var(--theme-color);
          color: #51b66d;

        }

        :hover .slide-box {
          opacity: 1;
          visibility: visible;
        }
        .slide-box {
          position: absolute;
          opacity: 0;
          visibility: hidden;
          transition: all 0.2s ease;
          inline-size: 100%;
          block-size: 100px;
          inset-block-end: 30px;
          border-start-start-radius: 9px;
          border-start-end-radius: 9px;
          background: var(--theme-color);
          display: flex;
          justify-content: center;
          align-items: center;
        }
        .slide-bg {
          position: relative;
          background: #99a7b2;
          inline-size: 4px;
          block-size: 80%;
          border-radius: 4px;
          transform: rotate(180deg);
        }
        .slide {
          block-size: 30px;
          inline-size: 4px;
          border-radius: 4px;
          position: absolute;
          transition: all 0.1s ease-in-out;
          background: #60bb7a;
        }
        .indicator {
          inline-size: 8px;
          block-size: 8px;
          display: block;
          inset-inline-start: 0;
          position: absolute;
          transition: all 0.1s ease-in-out;
          border-radius: 50%;
          background: white;
          transform: translateY(-25%) translateX(-25%);
        }
    "#
  ))
}
