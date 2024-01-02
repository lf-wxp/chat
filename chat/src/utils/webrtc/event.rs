#[macro_export]
macro_rules! bind_event {
  ($element:expr, $event_name:expr, $sender:expr, $message:path, $event_type:ty) => {{
    let message_callback = {
      let sender = $sender.clone();
      Closure::<dyn FnMut($event_type)>::new(move |ev| {
        let _ = sender.unbounded_send($message(ev));
      })
    };
    $element
      .add_event_listener_with_callback($event_name, message_callback.as_ref().unchecked_ref())
  }};
}
