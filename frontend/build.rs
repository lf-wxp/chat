use leptos_i18n_build::TranslationsInfos;
use leptos_i18n_parser::parse_locales::cfg_file::ConfigFile;
use std::path::PathBuf;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=Cargo.toml");

  let mut manifest_dir =
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

  let cfg_file = ConfigFile::new(&mut manifest_dir).expect("Failed to parse leptos-i18n config");
  let cfg: leptos_i18n_build::Config = cfg_file.into();

  let infos = TranslationsInfos::parse(cfg).expect("Failed to parse translations");

  infos.rerun_if_locales_changed();

  let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
  infos
    .generate_i18n_module(out_dir)
    .expect("Failed to generate i18n module");
}
