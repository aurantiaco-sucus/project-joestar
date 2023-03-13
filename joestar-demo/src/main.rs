use joestar::{Callback, launch_runtime, Model, Spec, View};
use joestar_html::{button, div, h1, p};

fn main() {
    env_logger::init();
    launch_runtime(user_main)
}

fn user_main() {
    let main = View::new(Spec {
        title: "Main".to_string(),
        size: (800, 600),
    });

    main.fill(&div()
        .with_child(h1("Hello World!"))
        .with_child(p("This is a paragraph."))
        .with_child(button("button1", "Click me!")));

    let div = main.root();
    div.set_style("background-color", "red");

    let button1 = main.lookup("button1");
    button1
        .bind("click", |event, detail| {
            println!("Button clicked!");
        });
}