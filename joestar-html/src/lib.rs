use joestar::Model;

pub fn div() -> Model {
    Model::new("div")
}

pub fn h1(text: &str) -> Model {
    Model::new("h1")
        .with_text(text)
}

pub fn p(text: &str) -> Model {
    Model::new("p")
        .with_text(text)
}

pub fn button(id: &str, text: &str) -> Model {
    Model::new("button")
        .with_id(id)
        .with_attr("type", "button")
        .with_text(text)
}

pub fn input(id: &str, input_type: &str) -> Model {
    Model::new("input")
        .with_id(id)
        .with_attr("type", input_type)
}
