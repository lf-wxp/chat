use stylist::{self, style};
use yew::prelude::*;

use crate::{components::Avatar, utils::{safe_slice, style}};

#[derive(Properties, PartialEq)]
pub struct Props {
  pub names: Vec<String>,
}

#[function_component]
pub fn AvatarMultitude(props: &Props) -> Html {
  let class_name = get_class_name();
  let slice = safe_slice(&props.names, 0, 9);
  let class = format!("{class_name} avatar-multi avatar-{}", slice.len());
  html! {
    <section {class}>
      { for slice.iter().map(|name| html! {
        <Avatar name={name.clone()} />
      })}
    </section>
  }
}
#[allow(non_upper_case_globals)]
fn get_class_name() -> String {
  style::get_class_name(style!(
    r#"
      background: var(--theme-color);
      border-radius: var(--radius);
      block-size: var(--avatar-size, 40px);
      inline-size: var(--avatar-size, 40px);
      border: 1px solid rgba(255, 255, 255, 0.1);
      display: grid;
      overflow: hidden;
      avatar {
        margin-inline-end: 0;
        border: none;
        border-radius: 0;
        overflow: hidden;
        --avatar-size: 100%;
      }
      &.avatar-3 {
        grid-template-rows: 1fr 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr 1fr;
      }
      &.avatar-3 avatar:nth-of-type(1) {
        grid-row: 1 / 3;
        grid-column: 2 / 4;
      }
      &.avatar-3 avatar:nth-of-type(2) {
        grid-row: 3 / 5;
        grid-column: 1 / 3;
      }
      &.avatar-3 avatar:nth-of-type(3) {
        grid-row: 3 / 5;
        grid-column: 3 / 5;
      }
      &.avatar-4 {
        grid-template-rows: 1fr 1fr;
        grid-template-columns: 1fr 1fr;
      }
      &.avatar-5 {
        grid-template-rows: 1fr 1fr 1fr 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr 1fr 1fr 1fr;
      }
      &.avatar-5 avatar:nth-of-type(1) {
        grid-row: 2 / 4;
        grid-column: 2 / 4;
      }
      &.avatar-5 avatar:nth-of-type(2) {
        grid-row: 2 / 4;
        grid-column: 4 / 6;
      }
      &.avatar-5 avatar:nth-of-type(3) {
        grid-row: 4 / 6;
        grid-column: 1 / 3;
      }
      &.avatar-5 avatar:nth-of-type(4) {
        grid-row: 4 / 6;
        grid-column: 3 / 5;
      }
      &.avatar-5 avatar:nth-of-type(5) {
        grid-row: 4 / 6;
        grid-column: 5 / 7;
      }
      &.avatar-6 {
        grid-template-rows: 1fr 1fr 1fr 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr 1fr 1fr 1fr;
      }
      &.avatar-6 avatar:nth-of-type(1) {
        grid-row: 2 / 4;
        grid-column: 1 / 3;
      }
      &.avatar-6 avatar:nth-of-type(2) {
        grid-row: 2 / 4;
        grid-column: 3 / 5;
      }
      &.avatar-6 avatar:nth-of-type(3) {
        grid-row: 2 / 4;
        grid-column: 5 / 7;
      }
      &.avatar-6 avatar:nth-of-type(4) {
        grid-row: 4 / 6;
        grid-column: 1 / 3;
      }
      &.avatar-6 avatar:nth-of-type(5) {
        grid-row: 4 / 6;
        grid-column: 3 / 5;
      }
      &.avatar-6 avatar:nth-of-type(6) {
        grid-row: 4 / 6;
        grid-column: 5 / 7;
      }
      &.avatar-7 {
        grid-template-rows: 1fr 1fr 1fr 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr 1fr 1fr 1fr;
      }
      &.avatar-7 avatar:nth-of-type(1) {
        grid-row: 1 / 3;
        grid-column: 3 / 5;
      }
      &.avatar-7 avatar:nth-of-type(2) {
        grid-row: 3 / 5;
        grid-column: 1 / 3;
      }
      &.avatar-7 avatar:nth-of-type(3) {
        grid-row: 3 / 5;
        grid-column: 3 / 5;
      }
      &.avatar-7 avatar:nth-of-type(4) {
        grid-row: 3 / 5;
        grid-column: 5 / 7;
      }
      &.avatar-7 avatar:nth-of-type(5) {
        grid-row: 5 / 7;
        grid-column: 1 / 3;
      }
      &.avatar-7 avatar:nth-of-type(6) {
        grid-row: 5 / 7;
        grid-column: 3 / 5;
      }
      &.avatar-7 avatar:nth-of-type(7) {
        grid-row: 5 / 7;
        grid-column: 5 / 7;
      }
      &.avatar-8 {
        grid-template-rows: 1fr 1fr 1fr 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr 1fr 1fr 1fr;
      }
      &.avatar-8 avatar:nth-of-type(1) {
        grid-row: 1 / 3;
        grid-column: 2 / 4;
      }
      &.avatar-8 avatar:nth-of-type(2) {
        grid-row: 1 / 3;
        grid-column: 4 / 6;
      }
      &.avatar-8 avatar:nth-of-type(3) {
        grid-row: 3 / 5;
        grid-column: 1 / 3;
      }
      &.avatar-8 avatar:nth-of-type(4) {
        grid-row: 3 / 5;
        grid-column: 3 / 5;
      }
      &.avatar-8 avatar:nth-of-type(5) {
        grid-row: 3 / 5;
        grid-column: 5 / 7;
      }
      &.avatar-8 avatar:nth-of-type(6) {
        grid-row: 5 / 7;
        grid-column: 1 / 3;
      }
      &.avatar-8 avatar:nth-of-type(7) {
        grid-row: 5 / 7;
        grid-column: 3 / 5;
      }
      &.avatar-8 avatar:nth-of-type(8) {
        grid-row: 5 / 7;
        grid-column: 5 / 7;
      }
      &.avatar-9 {
        grid-template-rows: 1fr 1fr 1fr;
        grid-template-columns: 1fr 1fr 1fr;
      }
    "#
  ))
}
