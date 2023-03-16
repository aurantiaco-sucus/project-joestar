mod rt;
mod api;

use std::collections::BTreeMap;
use wry::application::event_loop::{EventLoop};
use wry::webview::{WebView};

use rt::*;
pub use api::*;

/// Takes over the main thread and launch Joestar runtime.
///
/// Parameters:
/// * `user_init`: Initialization function to be invoked on user thread.
///
/// Remarks:
/// * It takes over the main thread and (likely) never returns.
/// * If a logger is to be set up, it should be done before calling this function.
#[inline]
pub fn launch_runtime(user_init: fn()) {
    let event_loop = EventLoop::<JoEvent>::with_user_event();
    let mut rt_state = RtState::new(event_loop.create_proxy());
    rt_state.proxy.send_event(JoEvent::UserLaunch { user_init }).unwrap();
    event_loop.run(move |
        event,
        window_target,
        control_flow
    | {
        handle_wry_event(
            &mut rt_state,
            event,
            window_target,
            control_flow
        );
    });
}