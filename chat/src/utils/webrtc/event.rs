use web_sys::RtcTrackEvent;


#[derive(Debug)]
pub enum MessageEvent {
  RtcTrackEvent(RtcTrackEvent) 
}
