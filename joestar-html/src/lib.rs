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

pub trait ModelLike : Sized {
    fn model(self) -> Model;
}

impl ModelLike for Model {
    fn model(self) -> Model {
        self
    }
}

pub trait ModelExt : ModelLike {
    fn display(self, kind: DisplayType) -> Model {
        self.model()
            .style("display", kind)
    }

    fn width(self, width: Length) -> Model {
        self.model()
            .style("width", width)
    }

    fn height(self, height: Length) -> Model {
        self.model()
            .style("height", height)
    }

    fn flex_grow(self, grow: u32) -> Model {
        self.model()
            .style("flex-grow", grow.to_string())
    }

    fn flex_shrink(self, shrink: u32) -> Model {
        self.model()
            .style("flex-shrink", shrink.to_string())
    }

    fn flex_fill(self) -> Model {
        self.model()
            .style("flex", "1")
    }
}

impl<T: ModelLike> ModelExt for T {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayType {
    Block,
    Flex,
    Inline,
    InlineBlock,
    InlineFlex,
    InlineTable,
    ListItem,
    None,
    RunIn,
    Table,
    TableCaption,
    TableCell,
    TableColumn,
    TableColumnGroup,
    TableFooterGroup,
    TableHeaderGroup,
    TableRow,
    TableRowGroup,
    TableLayoutAuto,
    TableLayoutFixed,
    Initial,
    Inherit,
}

impl From<DisplayType> for String {
    fn from(display: DisplayType) -> Self {
        match display {
            DisplayType::Block => "block",
            DisplayType::Flex => "flex",
            DisplayType::Inline => "inline",
            DisplayType::InlineBlock => "inline-block",
            DisplayType::InlineFlex => "inline-flex",
            DisplayType::InlineTable => "inline-table",
            DisplayType::ListItem => "list-item",
            DisplayType::None => "none",
            DisplayType::RunIn => "run-in",
            DisplayType::Table => "table",
            DisplayType::TableCaption => "table-caption",
            DisplayType::TableCell => "table-cell",
            DisplayType::TableColumn => "table-column",
            DisplayType::TableColumnGroup => "table-column-group",
            DisplayType::TableFooterGroup => "table-footer-group",
            DisplayType::TableHeaderGroup => "table-header-group",
            DisplayType::TableRow => "table-row",
            DisplayType::TableRowGroup => "table-row-group",
            DisplayType::TableLayoutAuto => "auto",
            DisplayType::TableLayoutFixed => "fixed",
            DisplayType::Initial => "initial",
            DisplayType::Inherit => "inherit",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Px(f32),
    Em(f32),
    Rem(f32),
    Percent(f32),
}

impl From<Length> for String {
    fn from(length: Length) -> Self {
        match length {
            Length::Px(px) => format!("{}px", px),
            Length::Em(em) => format!("{}em", em),
            Length::Rem(rem) => format!("{}rem", rem),
            Length::Percent(percent) => format!("{}%", percent),
        }
    }
}

pub trait AgentLike : Sized {
    fn as_agent(&self) -> &Agent;
}

impl AgentLike for Agent {
    fn as_agent(&self) -> &Agent {
        self
    }
}

pub trait AgentExt : AgentLike {
    fn on_click<F>(&self, f: F) -> Callback
        where
            F: Fn(ClickDetail) + 'static,
    {
        self.as_agent().bind("click", move |agent, detail| {
            let detail = ClickDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }

    fn on_input<F>(&self, f: F) -> Callback
        where
            F: Fn(ValueDetail) + 'static,
    {
        self.as_agent().bind("input", move |agent, detail| {
            let detail = ValueDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }

    fn on_change<F>(&self, f: F) -> Callback
        where
            F: Fn(ValueDetail) + 'static,
    {
        self.as_agent().bind("change", move |agent, detail| {
            let detail = ValueDetail::from_event(&detail).unwrap();
            f(detail);
        })
    }
}

impl<T: AgentLike> AgentExt for T {}

#[macro_export]
macro_rules! hflex {
    ($($x:expr),*$(,)?) => {
        div()
            .style("display", "flex")
            .style("flex-direction", "row")
            .children(vec![$($x),*])
    };
}

#[macro_export]
macro_rules! vflex {
    ($($x:expr),*$(,)?) => {
        div()
            .style("display", "flex")
            .style("flex-direction", "column")
            .children(vec![$($x),*])
    };
}