use std::collections::HashMap;
use std::hash::Hash;
use joestar::{Agent, Callback, Model};

/// Create a new division.
pub fn div() -> Model {
    Model::new("div")
}

/// Create a new 1st level heading.
pub fn h1(text: &str) -> Model {
    Model::new("h1")
        .text(text)
}

/// Create a new paragraph.
pub fn p(text: &str) -> Model {
    Model::new("p")
        .text(text)
}

/// Create a new button.
pub fn button(text: &str) -> Model {
    Model::new("button")
        .attr("type", "button")
        .text(text)
}

/// Create a new input.
pub fn input(input_type: &str) -> Model {
    Model::new("input")
        .attr("type", input_type)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

impl From<MouseButton> for u8 {
    fn from(button: MouseButton) -> Self {
        button as u8
    }
}

impl TryInto<MouseButton> for u8 {
    type Error = ();

    fn try_into(self) -> Result<MouseButton, Self::Error> {
        match self {
            0 => Ok(MouseButton::Left),
            1 => Ok(MouseButton::Middle),
            2 => Ok(MouseButton::Right),
            3 => Ok(MouseButton::Back),
            4 => Ok(MouseButton::Forward),
            _ => Err(()),
        }
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModifierStat {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClickDetail {
    pub button: MouseButton,
    pub position: (i32, i32),
    pub modifiers: ModifierStat,
}

impl ClickDetail {
    pub fn from_event(event: &HashMap<String, String>) -> Option<Self> {
        let button = event.get("button")?.parse::<u8>().ok()?;
        let button: MouseButton = button.try_into().ok()?;
        let position = (
            event.get("clientX")?.parse::<i32>().ok()?,
            event.get("clientY")?.parse::<i32>().ok()?,
        );
        let modifiers = ModifierStat {
            shift: event.get("shiftKey")?.parse::<bool>().ok()?,
            ctrl: event.get("ctrlKey")?.parse::<bool>().ok()?,
            alt: event.get("altKey")?.parse::<bool>().ok()?,
            meta: event.get("metaKey")?.parse::<bool>().ok()?,
        };
        Some(Self {
            button,
            position,
            modifiers,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ValueDetail {
    pub value: String,
}

impl ValueDetail {
    pub fn from_event(event: &HashMap<String, String>) -> Option<Self> {
        let value = event.get("likit_value")?.to_string();
        Some(Self { value })
    }
}

pub trait AgentExt {
    fn on_click<F>(&self, f: F) -> Callback
    where
        F: Fn(ClickDetail) + 'static;

    fn on_input<F>(&self, f: F) -> Callback
    where
        F: Fn(ValueDetail) + 'static;

    fn on_change<F>(&self, f: F) -> Callback
    where
        F: Fn(ValueDetail) + 'static;
}

impl AgentExt for Agent {
    fn on_click<F>(&self, f: F) -> Callback
    where
        F: Fn(ClickDetail) + 'static,
    {
        self.bind("click", move |agent, detail| {
            let detail = ClickDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }

    fn on_input<F>(&self, f: F) -> Callback
    where
        F: Fn(ValueDetail) + 'static,
    {
        self.bind("input", move |agent, detail| {
            let detail = ValueDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }

    fn on_change<F>(&self, f: F) -> Callback
    where
        F: Fn(ValueDetail) + 'static,
    {
        self.bind("change", move |agent, detail| {
            let detail = ValueDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }
}