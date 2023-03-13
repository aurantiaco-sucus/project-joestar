# Project Joestar

## What

`Joestar` is a [Wry](https://crates.io/crates/wry) based simple WebUI platform that provides a simple API for creation, management and observation of DOM elements. Suitable for basic cross-platform UIs that require no platform-related unsolvable bugs or unimplemented features (like IME support...).

## Why

### Another Toolkit

> There are AN AWFUL LOT of UI toolkits out there. You can even use GTK and Qt here with Rust, why bother creating another one?

**This is not another...**

* **...`qml-rs` or `gtk-rs`:** It's really small (not counting Wry) with only one source file.
* **...`imgui` or `egui`:** It's not immediate-mode UI.
* **...`relm4` or `dioxus`:** It does not implement a particular MVC system or follow a specific design pattern.

### Not Tauri

> It seems to be easier to just use `tauri` and write the UI things in ECMAScript. Why bother wrapping it into Rust and use it through an exotic interface?

If your App need a sophisticated UI that involve heavy theming and layout management, then you would want [`Next.js`](https://nextjs.org/) and [`Vite`](https://vitejs.dev/) with [`Tauri`](http://tauri.app) instead. Visit their websites to learn more about how you can take advantage of them.

`Joestar` is dead simple, to an extent that it's ONLY suitable for small apps that would just want to be clickable. It can theoretically do most things HTML5 and ECMA can, but it's not designed for that.

### Called Joestar

It's named after [Joseph Joestar](https://jojo.fandom.com/wiki/Joseph_Joestar), a famous JoJo's Bizarre Adventure character.

## How

Required packages are `joestar` and `joestar-html`.

```Rust
use joestar::{Callback, launch_runtime, Model, Spec, View};
use joestar_html::{AgentExt, button, div, h1, p};

fn main() {
    env_logger::init(); // Require env_logger
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
```