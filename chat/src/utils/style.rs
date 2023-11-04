use stylist::{Error, Style};

pub fn get_class_name(style: Result<Style, Error>) -> String {
  match style {
     Ok(style)  => style.get_class_name().to_owned(),
     Err(_) => "".to_owned()
  }
}
