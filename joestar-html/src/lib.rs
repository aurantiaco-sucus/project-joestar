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
pub fn button(id: &str, text: &str) -> Model {
    Model::new("button")
        .id(id)
        .attr("type", "button")
        .text(text)
}

/// Create a new input.
pub fn input(id: &str, input_type: &str) -> Model {
    Model::new("input")
        .id(id)
        .attr("type", input_type)
}

pub trait AgentExt {
    /// Bind a callback to click event.
    fn on_click<F>(&self, callback: F) -> Callback where F: FnMut(Agent) + 'static;
    /// Bind a callback to input event.
    fn on_input<F>(&self, callback: F) -> Callback where F: FnMut(Agent, String) + 'static;
}

impl AgentExt for Agent {
    fn on_click<F>(&self, mut callback: F) -> Callback where F: FnMut(Agent) + 'static {
        self.bind("click", move |agent, _| {
            callback(agent);
        })
    }

    fn on_input<F>(&self, mut callback: F) -> Callback where F: FnMut(Agent, String) + 'static {
        self.bind("input", move |agent, detail| {
            let value = detail.get("value").unwrap().to_string();
            callback(agent, value);
        })
    }
}