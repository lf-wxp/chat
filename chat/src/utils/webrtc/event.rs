#[macro_export]
macro_rules! bind_event {
  ($element:expr, $event_name:expr, $sender:expr, $message:path, $event_type:ty) => {{
    let message_callback = {
      let sender = $sender.clone();
      Closure::<dyn FnMut($event_type)>::new(move |ev| {
        let sender_clone = sender.clone();
        wasm_bindgen_futures::spawn_local(async move {
          let _ = sender_clone.broadcast_direct($message(ev)).await;
        });
      })
    };
    let _ = $element
      .add_event_listener_with_callback($event_name, message_callback.as_ref().unchecked_ref());
    message_callback.forget(); // 防止闭包在事件监听器结束时被销毁
  }};
}
