use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use log::error;
use wry::application::dpi::LogicalSize;
use wry::application::event::Event;
use wry::application::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use wry::application::window::{WindowBuilder, WindowId};
use wry::webview::{WebView, WebViewBuilder};

pub fn launch_runtime(user_init: fn()) {
    let event_loop = EventLoop::<JoEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut web_views: BTreeMap<usize, WebView> = BTreeMap::new();
    let mut web_views_from_id: BTreeMap<WindowId, usize> = BTreeMap::new();
    proxy.send_event(JoEvent::UserLaunch { user_init }).unwrap();
    event_loop.run(move |
        event,
        window_target,
        control_flow
    | {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { .. } => {}
            Event::DeviceEvent { .. } => {}
            Event::UserEvent(jo_event) => match jo_event {
                JoEvent::UserLaunch { user_init } => {
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
                JoEvent::CreateWebView { ord: window_id, spec } => {
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
                    web_views_from_id.insert(web_view.window().id(), window_id);
                    web_views.insert(window_id, web_view);
                }
                JoEvent::EvalScript { ord: window_id, script } => {
                    web_views.get(&window_id).unwrap()
                        .evaluate_script(&script).unwrap();
                }
                JoEvent::DestroyWebView { ord } => {
                    web_views.remove(&ord).unwrap();
                }
            },
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
    });
}

#[derive(Debug, Clone)]
enum JoEvent {
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
    static PROXY: RefCell<Option<EventLoopProxy<JoEvent>>> = RefCell::new(None);
    static SENDER: RefCell<Option<Sender<Box<dyn FnOnce()>>>> = RefCell::new(None);
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub title: String,
    pub size: (u32, u32),
}

static VIEW_ID_NEXT: AtomicUsize = AtomicUsize::new(0);
static mut VIEW_CUR: Vec<usize> = Vec::new();

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct View {
    ord: usize,
}

impl View {
    pub fn new(spec: Spec) -> Self {
        let ord = VIEW_ID_NEXT.fetch_add(1, Ordering::SeqCst);
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::CreateWebView {
                    ord,
                    spec,
                }).unwrap();
        });
        unsafe { VIEW_CUR.push(ord); }
        Self { ord }
    }

    pub fn acquire(id: usize) -> Option<Self> {
        unsafe { VIEW_CUR.get(id).map(|_| Self { ord: id }) }
    }

    pub fn eval(&self, script: String) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::EvalScript {
                    ord: self.ord,
                    script
                }).unwrap();
        });
    }

    pub fn destroy(self) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::DestroyWebView {
                    ord: self.ord,
                }).unwrap();
        });
        unsafe { VIEW_CUR.swap_remove(self.ord); }
    }

    pub fn fill(&self, model: &Model) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::EvalScript {
                    ord: self.ord,
                    script: format!("document.body.innerHTML = `{}`;", html_string(model))
                }).unwrap();
        });
    }

    pub fn ord(&self) -> usize {
        self.ord
    }

    pub fn root(&self) -> Agent {
        Agent {
            ord: self.ord,
            position: Position::Path(vec![]),
        }
    }

    pub fn lookup<S>(&self, id: S) -> Agent where S: Into<String> {
        Agent {
            ord: self.ord,
            position: Position::IdPath(id.into(), vec![]),
        }
    }
}

pub type WrappedCallback = Box<dyn FnMut(String, HashMap<String, String>)>;

pub struct Model {
    tag: String,
    id: Option<String>,
    attrs: HashMap<String, String>,
    style: HashMap<String, String>,
    text: Option<String>,
    children: Vec<Model>,
}

impl Model {
    pub fn new<S: Into<String>>(tag: S) -> Self {
        Self {
            tag: tag.into(),
            id: None,
            attrs: Default::default(),
            style: Default::default(),
            text: None,
            children: vec![],
        }
    }

    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_attr<S1: Into<String>, S2: Into<String>>(mut self, key: S1, val: S2) -> Self {
        self.attrs.insert(key.into(), val.into());
        self
    }

    pub fn with_style<S1: Into<String>, S2: Into<String>>(mut self, key: S1, val: S2) -> Self {
        self.style.insert(key.into(), val.into());
        self
    }

    pub fn with_child(mut self, child: Model) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = Some(text.into());
        self
    }

    fn export_markup(&self) -> String {
        html_string(self)
    }
}

fn attrs_string(attrs: &HashMap<String, String>) -> String {
    let mut attrs_string = String::new();
    for (key, val) in attrs {
        attrs_string.push_str(&format!("{}=\"{}\" ", key, val));
    }
    attrs_string
}

fn style_string(style: &HashMap<String, String>) -> String {
    let mut style_string = String::new();
    for (key, val) in style {
        style_string.push_str(&format!("{}: {}; ", key, val));
    }
    style_string
}

fn html_string(model: &Model) -> String {
    let mut result = String::new();
    result.push_str(&format!("<{}", model.tag));
    if let Some(id) = &model.id {
        result.push_str(&format!(" id=\"{}\"", id));
    }
    if !model.attrs.is_empty() {
        result.push_str(&format!(" {}", attrs_string(&model.attrs)));
    }
    if !model.style.is_empty() {
        result.push_str(&format!(" style=\"{}\"", style_string(&model.style)));
    }
    result.push_str(">");
    if let Some(text) = &model.text {
        result.push_str(&text);
    }
    for child in &model.children {
        result.push_str(&html_string(child));
    }
    result.push_str(&format!("</{}>", model.tag));
    result
}

fn invoke_callback(index: usize, path: &str, detail: HashMap<String, String>) {
    let callback = unsafe { CALLBACKS.get_mut(&index).unwrap() };
    callback(Agent::from(path), detail)
}

#[derive(Debug, Clone)]
pub enum Position {
    Path(Vec<usize>),
    IdPath(String, Vec<usize>),
}

#[derive(Debug, Clone)]
pub struct Agent {
    ord: usize,
    position: Position,
}

impl Agent {
    fn script_get_element(&self) -> String {
        match &self.position {
            Position::Path(path) => {
                let mut script = String::new();
                script.push_str(&format!("document.body.children[0]"));
                for i in path {
                    script.push_str(&format!(".children[{}]", i));
                }
                script
            }
            Position::IdPath(id, path) => {
                let mut script = String::new();
                script.push_str(&format!("document.getElementById(\"{}\")", id));
                for i in path {
                    script.push_str(&format!(".children[{}]", i));
                }
                script
            }
        }
    }

    fn script_set_element(&self, target: &str) -> String {
        let get_script = self.script_get_element();
        format!("{} = {}", get_script, target)
    }

    pub fn solve(&self, path: Vec<usize>) -> Self {
        let position = match &self.position {
            Position::Path(p) => Position::Path([&p[..], &path[..]].concat()),
            Position::IdPath(id, p) =>
                Position::IdPath(id.clone(), [&p[..], &path[..]].concat()),
        };
        Agent {
            ord: self.ord,
            position,
        }
    }

    pub fn view(&self) -> Option<View> {
        View::acquire(self.ord)
    }

    pub fn bind<F>(&self, key: &str, callback: F) -> Callback
    where
        F: FnMut(Agent, HashMap<String, String>) + 'static,
    {
        let callback = Callback::create(callback);
        let path: String = self.clone().into();
        let script = format!(
            "let elem = {};_lk_reg_evt(elem, \"{}\", \"{}\", \"{}\");",
            self.script_get_element(), key, path, callback.id,
        );
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
        callback
    }

    pub fn unbind(&self, key: &str) {
        let script = format!(
            "let elem = {};_lk_unreg_evt(elem, \"{}\");",
            self.script_get_element(), key,
        );
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
    }

    pub fn set(&self, key: &str, val: &str) {
        let script = format!(
            "{{let elem = {};elem.setAttribute(\"{}\", \"{}\");}}",
            self.script_get_element(), key, val,
        );
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
    }

    pub fn set_style(&self, key: &str, val: &str) {
        let script = format!(
            "{{let elem = {};elem.style.setProperty(\"{}\", \"{}\");}}",
            self.script_get_element(), key, val,
        );
        println!("{}", script);
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
    }
}

impl Into<String> for Agent {
    fn into(self) -> String {
        match self.position {
            Position::Path(path) => {
                let mut notation = format!("{}:", self.ord);
                for i in path {
                    notation.push_str(&format!("{},", i));
                }
                notation
            }
            Position::IdPath(id, path) => {
                let mut notation = format!("{},{}:", self.ord, id);
                for i in path {
                    notation.push_str(&format!("{},", i));
                }
                notation
            }
        }
    }
}

impl From<&str> for Agent {
    fn from(s: &str) -> Self {
        let mut s = s.trim().split(':');
        let head = s.next().unwrap();
        let tail = s.next().unwrap();
        let mut head = head.split(',');
        let ord = head.next().unwrap().parse::<usize>().unwrap();
        if let Some(id) = head.next() {
            let mut path = Vec::new();
            for i in tail.split(',') {
                if i.is_empty() { continue; }
                path.push(i.parse::<usize>().unwrap());
            }
            Agent {
                ord,
                position: Position::IdPath(id.to_string(), path),
            }
        } else {
            let mut path = Vec::new();
            for i in tail.split(',') {
                path.push(i.parse::<usize>().unwrap());
            }
            Agent {
                ord,
                position: Position::Path(path),
            }
        }
    }
}

pub struct Callback {
    id: usize,
}

type CallbackFunc = Box<dyn FnMut(Agent, HashMap<String, String>)>;

static mut CALLBACKS: BTreeMap<usize, CallbackFunc> = BTreeMap::new();
static CALLBACK_ID_NEXT: AtomicUsize = AtomicUsize::new(0);

impl Callback {
    pub fn create<F>(f: F) -> Self
    where
        F: FnMut(Agent, HashMap<String, String>) + 'static,
    {
        let id = CALLBACK_ID_NEXT.fetch_add(1, Ordering::SeqCst);
        unsafe {
            CALLBACKS.insert(id, Box::new(f));
        }
        Callback { id }
    }

    pub fn get(id: usize) -> Option<Self> {
        if unsafe { CALLBACKS.contains_key(&id) } {
            Some(Callback { id })
        } else {
            None
        }
    }

    pub fn remove(self) {
        unsafe {
            CALLBACKS.remove(&self.id);
        }
    }

    pub fn invoke(&self, agent: Agent, detail: HashMap<String, String>) {
        let callback = unsafe { CALLBACKS.get_mut(&self.id).unwrap() };
        callback(agent, detail)
    }
}