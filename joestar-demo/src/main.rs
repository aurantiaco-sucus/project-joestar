use joestar::{Callback, launch_runtime, Model, Spec, View};

fn main() {
    env_logger::init();
    launch_runtime(user_main)
}

fn user_main() {
    let main = View::new(Spec {
        title: "Main".to_string(),
        size: (800, 600),
    });

    let main_model = Model::new("div")
        .with_child(Model::new("h1")
            .with_text("Hello World!"))
        .with_child(Model::new("p")
            .with_text("This is a paragraph."))
        .with_child(Model::new("button")
            .with_id("button1")
            .with_attr("type", "button")
            .with_text("Click me!"));
    main.fill(&main_model);

    let div = main.root();
    div.set_style("background-color", "red");

    let button1 = main.lookup("button1");
    let button1_click = button1
        .bind("click", |event, detail| {
            println!("Button clicked!");
        });
}