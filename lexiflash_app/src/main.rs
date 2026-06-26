mod article_content;
mod components;
mod file_parser;
mod styles;
mod text_utils;
mod url_scraper;

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("LexiFlash")
                    .with_inner_size(LogicalSize::new(1160.0, 760.0))
                    .with_min_inner_size(LogicalSize::new(980.0, 660.0)),
            ),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        style { "{styles::APP_CSS}" }
        div { class: "app",
            components::create_deck::CreateDeckScreen {}
        }
    }
}
