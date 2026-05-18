use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        h1 { "subsecond hot-patch demo" }
        p { "count: {count}" }
        button { onclick: move |_| count += 1, "+1" }
        button { onclick: move |_| count -= 1, "-1" }
        hr {}
        p { style: "color: gray;",
            "edit src/main.rs while running `dx serve --hotpatch` and watch this page update without rebuild"
        }
    }
}
