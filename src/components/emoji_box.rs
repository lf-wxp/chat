use stylist::{self, style};
use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
  hook::use_click_exclusive,
  utils::{class_name_determine, style},
};

const EMOJI: [&str; 124] = [
  "😀",
  "😃",
  "😄",
  "😁",
  "😆",
  "😅",
  "😂",
  "🤣",
  "🥲",
  "🥹",
  "☺️",
  "😊",
  "😇",
  "🙂",
  "🙃",
  "😉",
  "😌",
  "😍",
  "🥰",
  "😘",
  "😗",
  "😙",
  "😚",
  "😋",
  "😛",
  "😝",
  "😜",
  "🤪",
  "🤨",
  "🧐",
  "🤓",
  "😎",
  "🥸",
  "🤩",
  "🥳",
  "😏",
  "😒",
  "😞",
  "😔",
  "😟",
  "😕",
  "🙁",
  "☹️",
  "😣",
  "😖",
  "😫",
  "😩",
  "🥺",
  "😢",
  "😭",
  "😮‍💨",
  "😤",
  "😠",
  "😡",
  "🤬",
  "🤯",
  "😳",
  "🥵",
  "🥶",
  "😱",
  "😨",
  "😰",
  "😥",
  "😓",
  "🫣",
  "🤗",
  "🫡",
  "🤔",
  "🫢",
  "🤭",
  "🤫",
  "🤥",
  "😶",
  "😶‍🌫️",
  "😐",
  "😑",
  "😬",
  "🫨",
  "🫠",
  "🙄",
  "😯",
  "😦",
  "😧",
  "😮",
  "😲",
  "🥱",
  "😴",
  "🤤",
  "😪",
  "😵",
  "😵‍💫",
  "🫥",
  "🤐",
  "🥴",
  "🤢",
  "🤮",
  "🤧",
  "😷",
  "🤒",
  "🤕",
  "🤑",
  "🤠",
  "😈",
  "👿",
  "👹",
  "👺",
  "🤡",
  "💩",
  "👻",
  "💀",
  "☠️",
  "👽",
  "👾",
  "🤖",
  "🎃",
  "😺",
  "😸",
  "😹",
  "😻",
  "😼",
  "😽",
  "🙀",
  "😿",
  "😾",
];

#[derive(Properties, PartialEq)]
pub struct Props {
  pub onclick: Callback<String>,
}

#[function_component]
pub fn EmojiBox(props: &Props) -> Html {
  let class_name = get_class_name();
  let visible = use_state(|| false);

  let onclick = {
    let visible = visible.clone();
    Callback::from(move |_| {
      visible.set(!*visible);
    })
  };

  let view_item = |item: &&str| {
    let onclick = props.onclick.clone();
    let call_item = item.to_string();
    html! {
      <li onclick={onclick.reform(move |_| call_item.clone())} class="emoji-item">{item}</li>
    }
  };

  let emoji_class = class_name_determine(*visible, "emoji", "active");

  let callback = {
    let visible = visible.clone();
    move || {
      visible.set(false);
    }
  };

  use_click_exclusive(vec![format!(".{}", class_name)], callback);

  html! {
    <div class={class_name}>
      <Icon {onclick} icon_id={IconId::BootstrapEmojiSmile} class={Classes::from(emoji_class)} width="16px" height="16px" />
      if *visible {
        <ul class="emoji-box">
        { for EMOJI.iter().map(view_item) }
        </ul>
      }
    </div>
  }
}

fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      inline-size: 16px;
      block-size: 16px;
      position: relative;
      .emoji {
        color: #8896a4;
        cursor: pointer;
        transition: all 0.2s ease;
      }
      .emoji:hover, .emoji.active {
        color: #51b66d;
      }
      .emoji-box {
        position: absolute;
        display: flex;
        flex-flow: row wrap;
        background: var(--theme-color);
        border-radius: var(--radius);
        inline-size: 300px;
      }
      .emoji-item {
        margin: 2px;
        padding-inline: 2px;
        cursor: pointer;
        transition: all 0.2s ease;
        border-radius: calc(var(--radius) / 3);
      }
      .emoji-item:hover {
        background: var(--theme-ancillary-color) ;
      }
    "#
  ))
}
