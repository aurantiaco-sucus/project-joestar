use joestar::{Callback, launch_runtime, Model, Spec, View};
use joestar_html::{AgentExt, button, div, h1, p};

fn main() {
    env_logger::init();
    launch_runtime(user_main)
}

fn user_main() {
    let main = View::new(Spec {
        title: "Main".to_string(),
        size: (800, 600),
    });

    main.fill(div()
        .children(vec![
            h1("Hello World!"),
            p("This is a paragraph."),
            button("button1", "Click me!"),
        ]));

    let div = main.root();
    div.set_style("background-color", "red");

    let button1 = main.lookup("button1");
    button1.on_click(|_| {
        println!("Button clicked!");
    });
}