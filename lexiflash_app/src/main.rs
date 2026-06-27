mod article_content;
mod components;
mod file_parser;
mod styles;
mod text_utils;
mod url_scraper;

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    CreateDeck,
    ReviewSession(Option<i64>),
}

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
    let mut screen = use_signal(|| Screen::Dashboard);

    rsx! {
        style { "{styles::APP_CSS}" }
        div { class: "app",
            match screen() {
                Screen::Dashboard => rsx! {
                    components::dashboard::Dashboard {
                        on_open_create_deck: move |_| screen.set(Screen::CreateDeck),
                        on_start_session: move |_| screen.set(Screen::ReviewSession(None)),
                    }
                },
                Screen::CreateDeck => rsx! {
                    components::create_deck::CreateDeckScreen {
                        on_show_dashboard: move |_| screen.set(Screen::Dashboard),
                    }
                },
                Screen::ReviewSession(deck_id) => rsx! {
                    components::review_session::ReviewSessionScreen {
                        deck_id,
                        on_show_dashboard: move |_| screen.set(Screen::Dashboard),
                    }
                },
            }
        }
    }
}
