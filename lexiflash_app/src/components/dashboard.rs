use dioxus::prelude::*;
use lexiflash_app::db::{self, DeckSummary, StudySnapshot};

#[derive(Clone, PartialEq, Eq)]
struct DashboardData {
    decks: Vec<DeckSummary>,
    stats: StudySnapshot,
}

#[derive(Clone, PartialEq, Eq)]
enum DashboardLoadState {
    Loaded(DashboardData),
    Error(String),
}

#[component]
pub fn Dashboard(on_open_create_deck: EventHandler<()>, on_start_session: EventHandler<()>) -> Element {
    let dashboard_state = use_signal(load_dashboard_state);

    let decks_hint = match dashboard_state() {
        DashboardLoadState::Loaded(data) => format!("{} total", data.decks.len()),
        DashboardLoadState::Error(_) => "Unavailable".to_string(),
    };

    let snapshot_body = match dashboard_state() {
        DashboardLoadState::Loaded(data) => rsx! {
            div { class: "stats_wrap",
                Stat { value: data.stats.learned_total.to_string(), label: "Learned total" }
                Stat { value: data.stats.streak_days.to_string(), label: "Day streak" }
                Stat { value: data.stats.due_today.to_string(), label: "Due today" }
            }
        },
        DashboardLoadState::Error(message) => rsx! {
            div { class: "dashboard_panel_body",
                div { class: "error_box",
                    "Không thể đọc dữ liệu Dashboard từ SQLite. {message}"
                }
            }
        },
    };

    let decks_body = match dashboard_state() {
        DashboardLoadState::Loaded(data) => {
            if data.decks.is_empty() {
                rsx! {
                    div { class: "empty_state deck_empty_state",
                        div { class: "eyebrow", "Library ready" }
                        div { class: "empty_title", "Chưa có deck nào trong thư viện cục bộ." }
                        div { class: "empty_copy",
                            "SQLite đã được khởi tạo đúng cách, nhưng hiện chưa có deck nào được lưu. Tạo deck đầu tiên của bạn để bắt đầu thư viện học tập và mở các phiên ôn bài FSRS."
                        }
                    }
                }
            } else {
                rsx! {
                    div { class: "deck_list",
                        for deck in data.decks {
                            DeckRow { deck }
                        }
                    }
                }
            }
        }
        DashboardLoadState::Error(message) => rsx! {
            div { class: "dashboard_panel_body",
                div { class: "error_box",
                    "Không thể mở hoặc truy vấn database cục bộ. {message}"
                }
            }
        },
    };

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
                        button {
                            class: "pill",
                            onclick: move |_| on_start_session.call(()),
                            span { "Start session" }
                            span { class: "pill_icon", "↗" }
                        }
                    }
                }

                main { class: "content",
                    section { class: "grid",
                        Card {
                            title: "Study snapshot",
                            hint: "Local DB",
                            style: "grid-column: 1 / span 2;",
                            children: snapshot_body
                        }

                        Card {
                            title: "Decks",
                            hint: decks_hint,
                            children: decks_body
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

fn load_dashboard_state() -> DashboardLoadState {
    match load_dashboard_data() {
        Ok(data) => DashboardLoadState::Loaded(data),
        Err(err) => DashboardLoadState::Error(format!("{err:#}")),
    }
}

fn load_dashboard_data() -> anyhow::Result<DashboardData> {
    let db_path = db::default_db_path()?;
    let conn = db::init_db(&db_path)?;
    let decks = db::list_decks(&conn)?;
    let stats = db::load_study_snapshot(&conn)?;
    Ok(DashboardData { decks, stats })
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
fn DeckRow(deck: DeckSummary) -> Element {
    rsx! {
        div { class: "deck_row",
            div { class: "deck_meta",
                div { class: "deck_title", "{deck.title}" }
                div { class: "deck_sub",
                    span { "{deck.created_at}" }
                    span { "·" }
                    span { "{deck.vocabulary_count} words" }
                    span { "·" }
                    span { "{deck.sentence_count} sentences" }
                }
            }
            div { class: "chip", "{deck.source_type}" }
        }
    }
}
