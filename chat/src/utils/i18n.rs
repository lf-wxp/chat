use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::HashMap;

lazy_static! {
  pub static ref TRANSLATIONS: HashMap<String, Value> = {
    let mut translations = HashMap::new();
    translations.insert(
      "en".to_string(),
      serde_json::json!({
          "input your name": "Input your name",
          "confirm": "confirm",
          "cancel": "cancel",
          "user name exist": "user name exist",
      }),
    );
    translations.insert(
      "zh".to_string(),
      serde_json::json!({
          "input your name": "请的输入名称",
          "confirm": "确认",
          "cancel": "取消",
          "user name exist": "用户名称已存在",
      }),
    );
    translations
  };
}
