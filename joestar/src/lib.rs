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

/// Takes over the main thread and launch Joestar runtime.
///
/// Parameters:
/// * `user_init`: Initialization function to be invoked on user thread.
///
/// Remarks:
/// * It takes over the main thread and (likely) never returns.
/// * If a logger is to be set up, it should be done before calling this function.
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

/// Configuration of a WebView.
///
/// Fields:
/// * `title`: Title of the window.
/// * `size`: Initial size of the window.
#[derive(Debug, Clone)]
pub struct Spec {
    pub title: String,
    pub size: (u32, u32),
}

static VIEW_ID_NEXT: AtomicUsize = AtomicUsize::new(0);
static mut VIEW_CUR: Vec<usize> = Vec::new();

/// Handle to a WebView.
///
/// Remarks:
/// * Only operate with the user runtime thread.
///     * This is enforced by the usage of a thread local static value of event loop proxy.
/// * The behaviour is not the same as plain old Rust stuffs.
///     * It doesn't destroy the WebView when it is dropped.
///     * In fact, the `acquire` function is used to gain access to a WebView through its index.
///     * You need to call `destroy` to dispose the WebView.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct View {
    ord: usize,
}

impl View {
    /// Create a new WebView.
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

    /// Acquire an existing WebView by its index.
    pub fn acquire(id: usize) -> Option<Self> {
        unsafe { VIEW_CUR.get(id).map(|_| Self { ord: id }) }
    }

    /// Evaluate arbitrary JavaScript code in the WebView.
    ///
    /// Remarks:
    /// * Safety concern: You need to know what you are doing.
    pub fn eval(&self, script: String) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::EvalScript {
                    ord: self.ord,
                    script
                }).unwrap();
        });
    }

    /// Destroy the WebView.
    pub fn destroy(self) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::DestroyWebView {
                    ord: self.ord,
                }).unwrap();
        });
        unsafe { VIEW_CUR.swap_remove(self.ord); }
    }

    /// Fill an element as the root node of content.
    pub fn fill(&self, model: Model) {
        PROXY.with(move |static_proxy| {
            static_proxy.borrow().as_ref().unwrap()
                .send_event(JoEvent::EvalScript {
                    ord: self.ord,
                    script: format!("document.body.innerHTML = `{}`;",
                                    html_string(&model))
                }).unwrap();
        });
    }

    /// Get the index of the WebView.
    pub fn ord(&self) -> usize {
        self.ord
    }

    /// Get the agent to the root node of content.
    pub fn root(&self) -> Agent {
        Agent {
            ord: self.ord,
            position: Position::Path(vec![]),
        }
    }

    /// Get the agent to an element by its ID.
    pub fn lookup<S>(&self, id: S) -> Agent where S: Into<String> {
        Agent {
            ord: self.ord,
            position: Position::IdPath(id.into(), vec![]),
        }
    }
}

pub type WrappedCallback = Box<dyn FnMut(String, HashMap<String, String>)>;

/// Model of a DOM element.
///
/// Remarks:
/// * All of the values are unchecked and not escaped, so be careful.
pub struct Model {
    tag: String,
    id: Option<String>,
    attrs: HashMap<String, String>,
    style: HashMap<String, String>,
    text: Option<String>,
    children: Vec<Model>,
}

impl Model {
    /// Create a new Model.
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

    /// Set the ID of the element.
    ///
    /// Remarks:
    /// * It does not check the correctness of the ID.
    pub fn id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set an attribute of the element.
    ///
    /// Remarks:
    /// * It does not check the correctness of the attribute.
    /// * It does not reject `style` or `id` attributes.
    pub fn attr<S1: Into<String>, S2: Into<String>>(mut self, key: S1, val: S2) -> Self {
        self.attrs.insert(key.into(), val.into());
        self
    }

    /// Set a style of the element.
    ///
    /// Remarks:
    /// * It does not check the correctness of the style.
    pub fn style<S1: Into<String>, S2: Into<String>>(mut self, key: S1, val: S2) -> Self {
        self.style.insert(key.into(), val.into());
        self
    }

    /// Add a child element.
    pub fn child(mut self, child: Model) -> Self {
        self.children.push(child);
        self
    }

    /// Add child elements.
    pub fn children(mut self, children: Vec<Model>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set the text content of the element.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
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

/// Position of an element.
///
/// Variants:
/// * Path: The path from the root node of content.
/// * IdPath: The path from the element with the given ID.
///
/// Remarks:
/// * The path is a sequence of indices of children.
/// * The path is empty for the root node of content or the element with the given ID.
#[derive(Debug, Clone)]
pub enum Position {
    Path(Vec<usize>),
    IdPath(String, Vec<usize>),
}

/// Agent to an element.
///
/// Remarks:
/// * The agent doesn't directly hold the element.
/// * It doesn't check the correctness of the path or ID.
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

    /// Get the agent to an element with path relative to the current element.
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

    /// Get the WebView.
    pub fn view(&self) -> Option<View> {
        View::acquire(self.ord)
    }

    /// Bind an callback to a DOM event.
    ///
    /// Remarks:
    /// * The callback is unique regarding to the event key.
    ///     * If the callback is already bound, it is replaced.
    /// * The callback is called with the agent to the element and the detail of the event.
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

    /// Unbind the callback to a DOM event.
    pub fn unbind(&self, key: &str) {
        let script = format!(
            "let elem = {};_lk_unreg_evt(elem, \"{}\");",
            self.script_get_element(), key,
        );
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
    }

    /// Set the specified attribute.
    pub fn set(&self, key: &str, val: &str) {
        let script = format!(
            "{{let elem = {};elem.setAttribute(\"{}\", \"{}\");}}",
            self.script_get_element(), key, val,
        );
        PROXY.with(move |proxy| proxy.borrow().as_ref().unwrap()
            .send_event(JoEvent::EvalScript { ord: self.ord, script }).unwrap());
    }

    /// Set the specified style.
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

/// A handle of callback to a DOM event.
pub struct Callback {
    id: usize,
}

type CallbackFunc = Box<dyn FnMut(Agent, HashMap<String, String>)>;

static mut CALLBACKS: BTreeMap<usize, CallbackFunc> = BTreeMap::new();
static CALLBACK_ID_NEXT: AtomicUsize = AtomicUsize::new(0);

impl Callback {
    /// Register a callback.
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

    /// Get a callback by its id.
    pub fn get(id: usize) -> Option<Self> {
        if unsafe { CALLBACKS.contains_key(&id) } {
            Some(Callback { id })
        } else {
            None
        }
    }

    /// Remove the callback from the registry.
    pub fn remove(self) {
        unsafe {
            CALLBACKS.remove(&self.id);
        }
    }

    /// Invoke the callback.
    pub fn invoke(&self, agent: Agent, detail: HashMap<String, String>) {
        let callback = unsafe { CALLBACKS.get_mut(&self.id).unwrap() };
        callback(agent, detail)
    }
}