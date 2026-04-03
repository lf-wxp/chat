use leptos_i18n_build::{Config, TranslationsInfos};
use std::path::PathBuf;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=Cargo.toml");
  println!("cargo:rerun-if-changed=locales");

  let cfg = Config::new("zh")
    .expect("Failed to create i18n config")
    .add_locale("en")
    .expect("Failed to add 'en' locale");

  let translations = TranslationsInfos::parse(cfg).expect("Failed to parse i18n translations");

  translations.rerun_if_locales_changed();

  let out_dir: PathBuf = std::env::var("OUT_DIR").expect("OUT_DIR not set").into();

  translations
    .generate_i18n_module(out_dir.join("i18n"))
    .expect("Failed to generate i18n module");
}
