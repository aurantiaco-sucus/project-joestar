use joestar::{Callback, joestar_terminate, launch_runtime, Model, Spec, View};
use joestar_html::{AgentExt, ModelExt, button, div, h1, hflex, input, p, vflex, Length};

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

    main.fill(vflex!(
        hflex!(
            h1("Hello World!")
                .style("background-color", "blue"),
            p("This is a paragraph.")
                .flex_fill(),
            div()
                .width(Length::Px(100.0))
                .height(Length::Px(100.0))
                .style("background-color", "red"),
        ),
        hflex!(
            button("Click me!")
                .id("button1"),
            input("text")
                .id("input1"),
        ),
    ));

    main.lookup("button1").on_click(|detail| {
        println!("Click: {:#?}", detail);
    });

    main.lookup("input1").on_input(|detail| {
        println!("Input: {:#?}", detail);
    });
}