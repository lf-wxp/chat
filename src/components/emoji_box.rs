use stylist::{self, style};
use yew::prelude::*;

use crate::utils::style;

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

  let view_item = |item: &&str| {
    let onclick = props.onclick.clone();
    let call_item = item.to_string();
    html! {
      <li onclick={onclick.reform(move |_| call_item.clone())} class="emoji-item">{item}</li>
    }
  };

  html! {
    <ul class={class_name}>
      { for EMOJI.iter().map(view_item) }
    </ul>
  }
}

#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      display: flex;
      flex-flow: row wrap;
      background: var(--theme-color);
      border-radius: var(--radius);
      inline-size: 300px;
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
