use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug};
use std::os::linux::raw::stat;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use log::error;
use wry::application::dpi::LogicalSize;
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoopProxy, EventLoopWindowTarget};
use wry::application::window::{WindowBuilder, WindowId};
use wry::webview::{WebView, WebViewBuilder};

use crate::api::*;

pub(crate) struct RtState {
    pub(crate) proxy: EventLoopProxy<JoEvent>,
    pub(crate) views: BTreeMap<usize, WebView>,
    pub(crate) view_event_callback_map: BTreeMap<usize, BTreeMap<ViewEventKey, usize>>,
    pub(crate) view_wid_map: BTreeMap<WindowId, usize>,
}

impl RtState {
    #[inline]
    pub(crate) fn new(proxy: EventLoopProxy<JoEvent>) -> Self {
        Self {
            proxy,
            views: BTreeMap::new(),
            view_event_callback_map: BTreeMap::new(),
            view_wid_map: BTreeMap::new(),
        }
    }
}

#[inline]
pub(crate) fn handle_wry_event(
    state: &mut RtState,
    event: Event<JoEvent>,
    window_target: &EventLoopWindowTarget<JoEvent>,
    control_flow: &mut ControlFlow
) {
    *control_flow = ControlFlow::Wait;
    match event {
        Event::NewEvents(_) => {}
        Event::WindowEvent { window_id, event , .. } => match event {
            WindowEvent::Resized(size) => {
                let ord = if let Some(ord) = state.view_wid_map.get(&window_id)
                { ord } else { return };
                let cb_index = if let Some(cbi) = state.view_event_callback_map
                    .get(ord).unwrap()
                    .get(&ViewEventKey::Resize)
                { cbi } else { return };
                let width = size.width;
                let height = size.height;
                let cb_index = *cb_index;
                user_dispatch(move || {
                    if let Some(cb) = Callback::get(cb_index) {
                        cb.invoke(Agent::invalid(), HashMap::from([
                            ("width".to_string(), width.to_string()),
                            ("height".to_string(), height.to_string()),
                        ]));
                    }
                })
            }
            WindowEvent::Moved(pos) => {
                let ord = if let Some(ord) = state.view_wid_map.get(&window_id)
                { ord } else { return };
                let cb_index = if let Some(cbi) = state.view_event_callback_map
                    .get(ord).unwrap()
                    .get(&ViewEventKey::Move)
                { cbi } else { return };
                let x = pos.x;
                let y = pos.y;
                let cb_index = *cb_index;
                user_dispatch(move || {
                    if let Some(cb) = Callback::get(cb_index) {
                        cb.invoke(Agent::invalid(), HashMap::from([
                            ("x".to_string(), x.to_string()),
                            ("y".to_string(), y.to_string()),
                        ]));
                    }
                })
            }
            WindowEvent::CloseRequested => {
                let ord = if let Some(ord) = state.view_wid_map.get(&window_id)
                { ord } else { return };
                let cb_index = if let Some(cbi) = state.view_event_callback_map
                    .get(ord).unwrap()
                    .get(&ViewEventKey::CloseRequest)
                { *cbi } else { return };
                user_dispatch(move || {
                    if let Some(cb) = Callback::get(cb_index) {
                        cb.invoke(Agent::invalid(), HashMap::new());
                    }
                })
            }
            WindowEvent::Destroyed => {}
            WindowEvent::DroppedFile(_) => {}
            WindowEvent::HoveredFile(_) => {}
            WindowEvent::HoveredFileCancelled => {}
            WindowEvent::ReceivedImeText(_) => {}
            WindowEvent::Focused(_) => {}
            WindowEvent::KeyboardInput { .. } => {}
            WindowEvent::ModifiersChanged(_) => {}
            WindowEvent::CursorMoved { .. } => {}
            WindowEvent::CursorEntered { .. } => {}
            WindowEvent::CursorLeft { .. } => {}
            WindowEvent::MouseWheel { .. } => {}
            WindowEvent::MouseInput { .. } => {}
            WindowEvent::TouchpadPressure { .. } => {}
            WindowEvent::AxisMotion { .. } => {}
            WindowEvent::Touch(_) => {}
            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::ThemeChanged(_) => {}
            WindowEvent::DecorationsClick => {}
            _ => {}
        }
        Event::DeviceEvent { .. } => {}
        Event::UserEvent(jo_event) => handle_joestar_event(
            state,
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
    state: &mut RtState,
    jo_event: JoEvent,
    window_target: &EventLoopWindowTarget<JoEvent>,
    control_flow: &mut ControlFlow
) {
    match jo_event {
        JoEvent::UserLaunch { user_init } =>
            handle_user_launch(user_init, &state.proxy),
        JoEvent::CreateWebView { ord, spec } =>
            handle_create_web_view(spec, ord, window_target, state),
        JoEvent::EvalScript { ord: window_id, script } => {
            state.views.get(&window_id).unwrap()
                .evaluate_script(&script).unwrap();
        }
        JoEvent::DestroyWebView { ord } => {
            state.views.remove(&ord).unwrap();
        }
        JoEvent::RegisterEvent { ord, key, cb_index } => {
            let callbacks = state.view_event_callback_map
                .entry(ord).or_default();
            callbacks.insert(key, cb_index);
        }
        JoEvent::Terminate => {
            *control_flow = ControlFlow::Exit;
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
        let (tx, rx) = mpsc::channel();
        init_sender(tx);
        user_init();
        while let Ok(callback) = rx.recv() {
            callback();
        }
    });
}

#[inline]
pub(crate) fn handle_create_web_view(
    spec: Spec,
    ord: usize,
    window_target: &EventLoopWindowTarget<JoEvent>,
    state: &mut RtState,
) {
    let window = WindowBuilder::new()
        .with_title(spec.title)
        .with_inner_size(LogicalSize::<u32>::from(spec.size))
        .build(window_target).unwrap();
    let web_view = WebViewBuilder::new(window).unwrap()
        .with_html(include_str!("index.html")).unwrap()
        .with_ipc_handler(|window, raw| {
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
                user_dispatch(move || {
                    callback.invoke(agent, detail);
                });
            }
        })
        .build().unwrap();
    let window_id = web_view.window().id();
    state.views.insert(ord, web_view);
    state.view_event_callback_map.insert(ord, BTreeMap::new());
    state.view_wid_map.insert(window_id, ord);
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
    RegisterEvent {
        ord: usize,
        key: ViewEventKey,
        cb_index: usize,
    },
    Terminate,
}

thread_local! {
    pub(crate) static PROXY: RefCell<Option<EventLoopProxy<JoEvent>>> = RefCell::new(None);
}

static mut SENDER: Option<Sender<Box<dyn FnOnce()>>> = None;

fn init_sender(sender: Sender<Box<dyn FnOnce()>>) {
    unsafe {
        SENDER = Some(sender);
    }
}

fn user_dispatch<F: FnOnce() + 'static>(f: F) {
    unsafe {
        SENDER.as_ref().unwrap().send(Box::new(f)).unwrap();
    }
}