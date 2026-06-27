use dioxus::prelude::*;

use crate::mock_data::{Deck, StudyStats};

#[component]
pub fn Dashboard(
    decks: Vec<Deck>,
    stats: StudyStats,
    on_open_create_deck: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "frame",
            div { class: "frame_inner",
                header { class: "topbar",
                    div { class: "brand",
                        div { class: "brand_title", "LexiFlash" }
                        div { class: "brand_subtitle", "Dashboard" }
                    }
                    div { class: "actions",
                        div { class: "pill_group",
                            button { class: "pill pill_active",
                                span { "Dashboard" }
                            }
                            button {
                                class: "pill",
                                onclick: move |_| on_open_create_deck.call(()),
                                span { "Create Deck" }
                                span { class: "pill_icon", "↗" }
                            }
                        }
                        div { class: "pill",
                            span { class: "pill_icon", "⌘" }
                            span { "Quick actions" }
                        }
                        div { class: "pill",
                            span { "Start session" }
                            span { class: "pill_icon", "↗" }
                        }
                    }
                }

                main { class: "content",
                    section { class: "grid",
                        Card {
                            title: "Study snapshot",
                            hint: "Today",
                            style: "grid-column: 1 / span 2;",
                            children: rsx! {
                                div { class: "stats_wrap",
                                    Stat { value: "{stats.learned_total}", label: "Learned total" }
                                    Stat { value: "{stats.streak_days}", label: "Day streak" }
                                    Stat { value: "{stats.due_today}", label: "Due today" }
                                }
                            }
                        }

                        Card {
                            title: "Decks",
                            hint: "{decks.len()} total",
                            children: rsx! {
                                div { class: "deck_list",
                                    for deck in decks {
                                        DeckRow { deck }
                                    }
                                }
                            }
                        }

                        Card {
                            title: "Create deck",
                            hint: "New",
                            children: rsx! {
                                div { class: "cta_card",
                                    div { class: "cta_inner",
                                        div { class: "cta_title", "Craft a deck with a clean input, clean intent." }
                                        div { class: "cta_copy",
                                            "Paste an article, import a file, or start from a single sentence. The pipeline is now wired into the desktop app."
                                        }
                                    }
                                    div {
                                        button {
                                            class: "cta_button",
                                            onclick: move |_| on_open_create_deck.call(()),
                                            span { "Create new deck" }
                                            span { class: "cta_trail", "↗" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Card(title: String, hint: String, children: Element, style: Option<String>) -> Element {
    rsx! {
        div { class: "card_shell", style: style.unwrap_or_default(),
            div { class: "card",
                div { class: "card_header",
                    div { class: "card_title", "{title}" }
                    div { class: "card_hint", "{hint}" }
                }
                {children}
            }
        }
    }
}

#[component]
fn Stat(value: String, label: String) -> Element {
    rsx! {
        div { class: "stat",
            div { class: "stat_value", "{value}" }
            div { class: "stat_label", "{label}" }
        }
    }
}

#[component]
fn DeckRow(deck: Deck) -> Element {
    rsx! {
        div { class: "deck_row",
            div { class: "deck_meta",
                div { class: "deck_title", "{deck.title}" }
                div { class: "deck_sub",
                    span { "{deck.created_at}" }
                    span { "·" }
                    span { "{deck.vocab_count} words" }
                }
            }
            div { class: "chip", "Open" }
        }
    }
}
