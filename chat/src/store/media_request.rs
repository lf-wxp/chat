use bounce::Atom;
use message::MediaMessage;

#[derive(Atom, PartialEq, Default)]
pub struct MediaRequest(pub Option<MediaMessage>);
