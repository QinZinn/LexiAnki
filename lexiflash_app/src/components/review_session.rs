use dioxus::prelude::*;
use lexiflash_app::db::{self, DueCard};
use lexiflash_app::fsrs_scheduler::{self, ReviewRating};

#[derive(Clone, PartialEq)]
enum ReviewLoadState {
    Loaded(Vec<DueCard>),
    Error(String),
}

#[component]
pub fn ReviewSessionScreen(deck_id: Option<i64>, on_show_dashboard: EventHandler<()>) -> Element {
    let mut review_state = use_signal(|| load_due_cards(deck_id));
    let mut reveal_back = use_signal(|| false);
    let mut reviewed_count = use_signal(|| 0usize);
    let mut action_error = use_signal(|| None::<String>);

    let remaining = match review_state() {
        ReviewLoadState::Loaded(cards) => cards.len(),
        ReviewLoadState::Error(_) => 0,
    };

    let current_card = match review_state() {
        ReviewLoadState::Loaded(cards) => cards.first().cloned(),
        ReviewLoadState::Error(_) => None,
    };

    let mut submit_again = {
        let current_card = current_card.clone();
        move |_| {
            apply_rating(
                &mut review_state,
                &mut reviewed_count,
                &mut reveal_back,
                &mut action_error,
                current_card.clone(),
                ReviewRating::Again,
            )
        }
    };
    let mut submit_hard = {
        let current_card = current_card.clone();
        move |_| {
            apply_rating(
                &mut review_state,
                &mut reviewed_count,
                &mut reveal_back,
                &mut action_error,
                current_card.clone(),
                ReviewRating::Hard,
            )
        }
    };
    let mut submit_good = {
        let current_card = current_card.clone();
        move |_| {
            apply_rating(
                &mut review_state,
                &mut reviewed_count,
                &mut reveal_back,
                &mut action_error,
                current_card.clone(),
                ReviewRating::Good,
            )
        }
    };
    let mut submit_easy = {
        let current_card = current_card.clone();
        move |_| {
            apply_rating(
                &mut review_state,
                &mut reviewed_count,
                &mut reveal_back,
                &mut action_error,
                current_card.clone(),
                ReviewRating::Easy,
            )
        }
    };

    rsx! {
        div { class: "frame",
            div { class: "frame_inner",
                header { class: "topbar",
                    div { class: "brand",
                        div { class: "brand_title", "LexiFlash" }
                        div { class: "brand_subtitle", "Review Session" }
                    }
                    div { class: "actions",
                        div { class: "pill_group",
                            button {
                                class: "pill",
                                onclick: move |_| on_show_dashboard.call(()),
                                span { "Dashboard" }
                            }
                            button { class: "pill pill_active",
                                span { "Review Session" }
                            }
                        }
                        if matches!(review_state(), ReviewLoadState::Loaded(_)) {
                            div { class: "pill",
                                span { "{remaining} due" }
                            }
                        }
                    }
                }

                main { class: "content",
                    section { class: "review_layout",
                        match review_state() {
                            ReviewLoadState::Error(message) => rsx! {
                                div { class: "review_shell",
                                    div { class: "review_panel review_feedback",
                                        div { class: "eyebrow", "Session unavailable" }
                                        div { class: "review_title", "Không thể mở phiên ôn bài." }
                                        div { class: "error_box", "{message}" }
                                        button {
                                            class: "cta_button",
                                            onclick: move |_| on_show_dashboard.call(()),
                                            span { "Về Dashboard" }
                                            span { class: "cta_trail", "↗" }
                                        }
                                    }
                                }
                            },
                            ReviewLoadState::Loaded(cards) => {
                                if let Some(card) = current_card {
                                    let pos_label = card
                                        .entry
                                        .wordnet_pos
                                        .map(|value| format!("{value:?}"))
                                        .unwrap_or_else(|| "None".to_string());

                                    rsx! {
                                        div { class: "review_shell",
                                            div {
                                                class: if reveal_back() { "review_panel review_card_panel review_card_revealed" } else { "review_panel review_card_panel" },
                                                onclick: move |_| reveal_back.set(!reveal_back()),
                                                div { class: "eyebrow", "Tap to flip" }
                                                div { class: "review_title", "{card.entry.lemma}" }
                                                div { class: "review_meta",
                                                    div { class: "chip", "{card.deck_title}" }
                                                    div { class: "chip", "{card.review_state.state:?}" }
                                                }
                                                if reveal_back() {
                                                    div { class: "review_back",
                                                        div { class: "review_context", "{card.entry.context}" }
                                                        div { class: "review_detail_row",
                                                            div { class: "review_detail_label", "Original token" }
                                                            div { class: "review_detail_value", "{card.entry.original_token}" }
                                                        }
                                                        div { class: "review_detail_row",
                                                            div { class: "review_detail_label", "WordNet POS" }
                                                            div { class: "review_detail_value", "{pos_label}" }
                                                        }
                                                    }
                                                } else {
                                                    div { class: "review_prompt",
                                                        "Giữ sự tập trung ở một thẻ mỗi lần. Khi đã nhớ lại, chạm để mở context rồi chấm Again / Hard / Good / Easy."
                                                    }
                                                }
                                            }

                                            div { class: "review_panel review_feedback",
                                                div { class: "review_progress",
                                                    div { class: "stat_value", "{cards.len()}" }
                                                    div { class: "stat_label", "Cards Remaining" }
                                                }
                                                div { class: "review_copy",
                                                    if reveal_back() {
                                                        "Chọn đúng mức độ ghi nhớ để FSRS đẩy due date mới ra tương lai."
                                                    } else {
                                                        "Lật thẻ trước khi chấm rating để giữ trải nghiệm ôn tập rõ ràng và nhất quán."
                                                    }
                                                }
                                                if let Some(message) = action_error() {
                                                    div { class: "error_box", "{message}" }
                                                }
                                                div { class: "rating_row",
                                                    button {
                                                        class: "rating_button rating_again",
                                                        disabled: !reveal_back(),
                                                        onclick: move |evt| submit_again(evt),
                                                        "Again"
                                                    }
                                                    button {
                                                        class: "rating_button rating_hard",
                                                        disabled: !reveal_back(),
                                                        onclick: move |evt| submit_hard(evt),
                                                        "Hard"
                                                    }
                                                    button {
                                                        class: "rating_button rating_good",
                                                        disabled: !reveal_back(),
                                                        onclick: move |evt| submit_good(evt),
                                                        "Good"
                                                    }
                                                    button {
                                                        class: "rating_button rating_easy",
                                                        disabled: !reveal_back(),
                                                        onclick: move |evt| submit_easy(evt),
                                                        "Easy"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "review_shell",
                                            div { class: "review_panel review_feedback review_complete",
                                                div { class: "eyebrow", "All clear" }
                                                div { class: "review_title", "Đã ôn xong {reviewed_count()} thẻ hôm nay." }
                                                div { class: "review_copy",
                                                    "Tất cả thẻ đang due đã được đẩy sang mốc thời gian mới theo FSRS. Quay lại Dashboard để xem Due Today giảm xuống."
                                                }
                                                button {
                                                    class: "cta_button",
                                                    onclick: move |_| on_show_dashboard.call(()),
                                                    span { "Về Dashboard" }
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
    }
}

fn load_due_cards(deck_id: Option<i64>) -> ReviewLoadState {
    match load_due_cards_from_db(deck_id) {
        Ok(cards) => ReviewLoadState::Loaded(cards),
        Err(err) => ReviewLoadState::Error(format!("{err:#}")),
    }
}

fn load_due_cards_from_db(deck_id: Option<i64>) -> anyhow::Result<Vec<DueCard>> {
    let db_path = db::default_db_path()?;
    let conn = db::init_db(&db_path)?;
    db::get_due_cards(&conn, deck_id)
}

fn submit_rating(card: &DueCard, rating: ReviewRating) -> anyhow::Result<()> {
    let db_path = db::default_db_path()?;
    let conn = db::init_db(&db_path)?;
    let next_state = fsrs_scheduler::schedule_review(&card.review_state, rating)?;
    db::update_review_state(&conn, card.vocabulary_entry_id, &next_state)
}

fn apply_rating(
    review_state: &mut Signal<ReviewLoadState>,
    reviewed_count: &mut Signal<usize>,
    reveal_back: &mut Signal<bool>,
    action_error: &mut Signal<Option<String>>,
    current_card: Option<DueCard>,
    rating: ReviewRating,
) {
    action_error.set(None);

    let Some(card) = current_card else {
        return;
    };

    match submit_rating(&card, rating) {
        Ok(()) => {
            if let ReviewLoadState::Loaded(mut cards) = review_state() {
                if !cards.is_empty() {
                    cards.remove(0);
                }
                review_state.set(ReviewLoadState::Loaded(cards));
                reviewed_count.set(reviewed_count() + 1);
                reveal_back.set(false);
            }
        }
        Err(err) => action_error.set(Some(format!("{err:#}"))),
    }
}
