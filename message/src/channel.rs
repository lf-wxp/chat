#[macro_export]
macro_rules! channel {
  ($struct_name:ident, $message_type:ty, $message_type_path:path, $response_type:ty, $response_type_path:path) => {
    #[derive(Clone)]
    pub struct $struct_name {
      sender: async_broadcast::Sender<gloo_net::websocket::Message>,
      receiver: async_broadcast::Receiver<gloo_net::websocket::Message>,
    }

    impl $struct_name {
      pub fn new(
        sender: async_broadcast::Sender<gloo_net::websocket::Message>,
        receiver: async_broadcast::Receiver<gloo_net::websocket::Message>,
      ) -> Self {
        $struct_name { sender, receiver }
      }

      pub fn send(&mut self, message: $message_type) {
        let message = $message_type_path(message);
        let message = serde_json::to_string(&message).unwrap();
        self
          .sender
          .broadcast(gloo_net::websocket::Message::Text(message));
      }
    }

    impl futures::Stream for $struct_name {
      type Item = $response_type;

      fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
      ) -> std::task::Poll<Option<Self::Item>> {
        let msg = futures::ready!(futures::StreamExt::poll_next_unpin(
          &mut self.get_mut().receiver,
          cx
        ));
        match msg {
          Some(gloo_net::websocket::Message::Text(msg)) => {
            match serde_json::from_str::<ResponseMessage>(&msg) {
              Ok(msg) => {
                if let $response_type_path(msg) = msg {
                  return std::task::Poll::Ready(Some(msg));
                }
                return std::task::Poll::Pending;
              }
              Err(_) => std::task::Poll::Ready(None),
            }
          }
          _ => std::task::Poll::Ready(None),
        }
      }
    }
  };
}
