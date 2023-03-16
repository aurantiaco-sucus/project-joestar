use joestar::{Callback, joestar_terminate, launch_runtime, Model, Spec, View};
use joestar_html::{AgentExt, button, div, h1, input, p};

fn main() {
    env_logger::init();
    launch_runtime(user_main)
}

fn user_main() {
    let main = View::new(Spec {
        title: "Main".to_string(),
        size: (800, 600),
    });
    let main_ord = main.ord();

    main.on_close_request(move || {
        println!("See you next time!");
        View::acquire(main_ord).unwrap().destroy();
        joestar_terminate();
    });

    main.fill(div()
        .children(vec![
            h1("Hello World!"),
            p("This is a paragraph."),
            button("Click me!")
                .id("button1"),
            input("text")
                .id("input1"),
        ]));

    main.lookup("button1").on_click(|detail| {
        println!("Click: {:#?}", detail);
    });

    main.lookup("input1").on_input(|detail| {
        println!("Input: {:#?}", detail);
    });
}