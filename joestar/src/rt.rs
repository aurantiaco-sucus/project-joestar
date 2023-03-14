use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use log::error;
use wry::application::dpi::LogicalSize;
use wry::application::event::Event;
use wry::application::event_loop::{ControlFlow, EventLoopProxy, EventLoopWindowTarget};
use wry::application::window::{WindowBuilder};
use wry::webview::{WebView, WebViewBuilder};

use crate::api::*;

#[inline]
pub(crate) fn handle_wry_event(
    web_views: &mut BTreeMap<usize, WebView>,
    proxy: &EventLoopProxy<JoEvent>,
    event: Event<JoEvent>,
    window_target: &EventLoopWindowTarget<JoEvent>,
    control_flow: &mut ControlFlow
) {
    *control_flow = ControlFlow::Wait;
    match event {
        Event::NewEvents(_) => {}
        Event::WindowEvent { .. } => {}
        Event::DeviceEvent { .. } => {}
        Event::UserEvent(jo_event) => handle_joestar_event(
            web_views,
            proxy,
            jo_event,
            window_target,
            control_flow
        ),
        Event::MenuEvent { .. } => {}
        Event::TrayEvent { .. } => {}
        Event::GlobalShortcutEvent(_) => {}
        Event::Suspended => {}
        Event::Resumed => {}
        Event::MainEventsCleared => {}
        Event::RedrawRequested(_) => {}
        Event::RedrawEventsCleared => {}
        Event::LoopDestroyed => {}
        _ => {
            error!("Unknown event: {event:?}");
        }
    }
}

#[inline]
pub(crate) fn handle_joestar_event(
    web_views: &mut BTreeMap<usize, WebView>,
    proxy: &EventLoopProxy<JoEvent>,
    jo_event: JoEvent,
    window_target: &EventLoopWindowTarget<JoEvent>,
    control_flow: &mut ControlFlow
) {
    match jo_event {
        JoEvent::UserLaunch { user_init } =>
            handle_user_launch(user_init, proxy),
        JoEvent::CreateWebView { ord, spec } =>
            handle_create_web_view(spec, ord, window_target, web_views),
        JoEvent::EvalScript { ord: window_id, script } => {
            web_views.get(&window_id).unwrap()
                .evaluate_script(&script).unwrap();
        }
        JoEvent::DestroyWebView { ord } => {
            web_views.remove(&ord).unwrap();
        }
    }
}

#[inline]
pub(crate) fn handle_user_launch(user_init: fn(), proxy: &EventLoopProxy<JoEvent>) {
    let proxy = proxy.clone();
    thread::spawn(move || {
        PROXY.with(move |static_proxy| {
            *static_proxy.borrow_mut() = Some(proxy);
        });
        let (sender, receiver) = mpsc::channel();
        SENDER.with(move |sender_cell| {
            *sender_cell.borrow_mut() = Some(sender);
        });
        user_init();
        while let Ok(callback) = receiver.recv() {
            callback();
        }
    });
}

#[inline]
pub(crate) fn handle_create_web_view(
    spec: Spec,
    ord: usize,
    window_target: &EventLoopWindowTarget<JoEvent>,
    web_views: &mut BTreeMap<usize, WebView>
) {
    let window = WindowBuilder::new()
        .with_title(spec.title)
        .with_inner_size(LogicalSize::<u32>::from(spec.size))
        .build(window_target).unwrap();
    let web_view = WebViewBuilder::new(window).unwrap()
        .with_html(include_str!("index.html")).unwrap()
        .with_ipc_handler(|window, raw| {
            if raw.starts_with('$') {
                let raw = &raw[1..];
                let mut raw = raw.split(':');
                let cmd = raw.next().unwrap();
                let arg = raw.next().unwrap();
                match cmd {
                    "dedup" => {
                        let n = arg.parse().unwrap();
                        Callback::get(n).unwrap().remove();
                    }
                    "@" => {}
                    _ => {}
                }
            }
            let mut raw = raw.lines();
            let head = raw.next().unwrap();
            let mut head = head.split(">>>");
            let path = head.next().unwrap();
            let agent = Agent::from(path);
            let cb_index: usize = head.next().unwrap().parse().unwrap();
            let mut detail: HashMap<String, String> = HashMap::new();
            while let Some(key) = raw.next() {
                let value = raw.next().unwrap();
                detail.insert(key.into(), value.into());
            }
            if let Some(callback) = Callback::get(cb_index) {
                callback.invoke(agent, detail);
            }
        })
        .build().unwrap();
    web_views.insert(ord, web_view);
}

#[derive(Debug, Clone)]
pub(crate) enum JoEvent {
    UserLaunch {
        user_init: fn(),
    },
    CreateWebView {
        ord: usize,
        spec: Spec,
    },
    EvalScript {
        ord: usize,
        script: String,
    },
    DestroyWebView {
        ord: usize,
    },
}

thread_local! {
    pub(crate) static PROXY: RefCell<Option<EventLoopProxy<JoEvent>>> = RefCell::new(None);
    pub(crate) static SENDER: RefCell<Option<Sender<Box<dyn FnOnce()>>>> = RefCell::new(None);
}