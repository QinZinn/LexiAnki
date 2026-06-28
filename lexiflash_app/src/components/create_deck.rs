use std::path::Path;

use dioxus::prelude::*;
use lexiflash_app::db;
use lexiflash_nlp::{LexiFlashNlp, VocabularyEntry};
use rfd::FileDialog;

use crate::article_content::ArticleContent;
use crate::file_parser;
use crate::url_scraper;

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Url,
    File,
}

#[derive(Clone, PartialEq, Eq)]
struct DeckPreview {
    title: String,
    source: String,
    sentence_count: usize,
    vocabulary: Vec<VocabularyEntry>,
}

enum CreateDeckResult {
    Saved { preview: DeckPreview, deck_id: i64 },
    SaveFailed { preview: DeckPreview, message: String },
}

#[derive(Clone, PartialEq, Eq)]
enum SaveNotice {
    Success { deck_id: i64, message: String },
    Error(String),
}

#[component]
pub fn CreateDeckScreen(on_show_dashboard: EventHandler<()>) -> Element {
    let mut mode = use_signal(|| InputMode::Url);
    let mut url_input = use_signal(String::new);
    let mut selected_file = use_signal(|| None::<String>);
    let mut result = use_signal(|| None::<DeckPreview>);
    let mut error = use_signal(|| None::<String>);
    let mut save_notice = use_signal(|| None::<SaveNotice>);
    let mut is_processing_url = use_signal(|| false);

    let mut process_url = {
        move || {
            error.set(None);
            result.set(None);
            save_notice.set(None);
            is_processing_url.set(true);

            let url = url_input();
            let mut result = result;
            let mut error = error;
            let mut save_notice = save_notice;
            let mut is_processing_url = is_processing_url;

            spawn(async move {
                let outcome = process_from_url(&url).await;
                is_processing_url.set(false);

                match outcome {
                    Ok(CreateDeckResult::Saved { preview, deck_id }) => {
                        let vocab_count = preview.vocabulary.len();
                        result.set(Some(preview));
                        save_notice.set(Some(SaveNotice::Success {
                            deck_id,
                            message: format!("Đã lưu deck vào SQLite — {vocab_count} từ vựng."),
                        }));
                    }
                    Ok(CreateDeckResult::SaveFailed { preview, message }) => {
                        result.set(Some(preview));
                        save_notice.set(Some(SaveNotice::Error(message)));
                    }
                    Err(err) => error.set(Some(err.to_string())),
                }
            });
        }
    };

    let mut process_file = {
        move || {
            error.set(None);
            result.set(None);
            save_notice.set(None);

            let Some(path) = selected_file() else {
                error.set(Some("Chưa chọn file đầu vào.".to_string()));
                return;
            };

            match process_from_file(&path) {
                Ok(CreateDeckResult::Saved { preview, deck_id }) => {
                    let vocab_count = preview.vocabulary.len();
                    result.set(Some(preview));
                    save_notice.set(Some(SaveNotice::Success {
                        deck_id,
                        message: format!("Đã lưu deck vào SQLite — {vocab_count} từ vựng."),
                    }));
                }
                Ok(CreateDeckResult::SaveFailed { preview, message }) => {
                    result.set(Some(preview));
                    save_notice.set(Some(SaveNotice::Error(message)));
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        }
    };

    let mut pick_file = {
        move || {
            if let Some(path) = FileDialog::new()
                .add_filter("Supported documents", &["txt", "docx", "pptx", "pdf"])
                .pick_file()
            {
                selected_file.set(Some(path.display().to_string()));
                error.set(None);
            }
        }
    };

    rsx! {
        div { class: "frame",
            div { class: "frame_inner",
                header { class: "topbar",
                    div { class: "brand",
                        div { class: "brand_title", "LexiFlash" }
                        div { class: "brand_subtitle", "Create Deck" }
                    }
                    div { class: "actions",
                        div { class: "pill_group",
                            button {
                                class: "pill",
                                onclick: move |_| on_show_dashboard.call(()),
                                span { "Dashboard" }
                            }
                            button { class: "pill pill_active",
                                span { "Create Deck" }
                            }
                        }
                        div { class: "pill_group",
                            button {
                                class: if matches!(mode(), InputMode::Url) { "pill pill_active" } else { "pill" },
                                onclick: move |_| mode.set(InputMode::Url),
                                span { "From URL" }
                            }
                            button {
                                class: if matches!(mode(), InputMode::File) { "pill pill_active" } else { "pill" },
                                onclick: move |_| mode.set(InputMode::File),
                                span { "From File" }
                            }
                        }
                    }
                }

                main { class: "content",
                    section { class: "create_grid",
                        div { class: "card_shell create_input_shell",
                            div { class: "card create_input_card",
                                div { class: "card_header",
                                    div { class: "card_title", "Source" }
                                    div { class: "card_hint",
                                        if matches!(mode(), InputMode::Url) { "Live fetch" } else { "Local file" }
                                    }
                                }

                                div { class: "create_body",
                                    div { class: "eyebrow", "Choose an input path" }
                                    div { class: "create_intro",
                                        "Paste an article URL or open a local document, then run the full Rust pipeline to preview extracted vocabulary."
                                    }

                                    if matches!(mode(), InputMode::Url) {
                                        div { class: "field_group",
                                            label { class: "field_label", "Article URL" }
                                            input {
                                                class: "text_input",
                                                r#type: "text",
                                                value: "{url_input}",
                                                placeholder: "https://www.bbc.com/news/articles/...",
                                                oninput: move |evt| url_input.set(evt.value()),
                                            }
                                        }
                                        div { class: "action_row",
                                            button {
                                                class: "cta_button",
                                                disabled: is_processing_url(),
                                                onclick: move |_| process_url(),
                                                span {
                                                    if is_processing_url() { "Extracting..." } else { "Extract from URL" }
                                                }
                                                span { class: "cta_trail", "↗" }
                                            }
                                        }
                                    } else {
                                        div { class: "field_group",
                                            label { class: "field_label", "Selected file" }
                                            div { class: "file_pick_row",
                                                button {
                                                    class: "pill",
                                                    onclick: move |_| pick_file(),
                                                    span { "Choose file" }
                                                    span { class: "pill_icon", "↗" }
                                                }
                                                div { class: "file_path" ,
                                                    {selected_file().unwrap_or_else(|| "No file selected".to_string())}
                                                }
                                            }
                                        }
                                        div { class: "action_row",
                                            button {
                                                class: "cta_button",
                                                onclick: move |_| process_file(),
                                                span { "Extract from file" }
                                                span { class: "cta_trail", "↗" }
                                            }
                                        }
                                    }

                                    if let Some(message) = error() {
                                        div { class: "error_box", "{message}" }
                                    }

                                    if let Some(notice) = save_notice() {
                                        match notice {
                                            SaveNotice::Success { deck_id, message } => rsx! {
                                                div { class: "success_box",
                                                    div { class: "status_copy", "{message}" }
                                                    div { class: "status_row",
                                                        div { class: "chip", "Deck #{deck_id}" }
                                                        button {
                                                            class: "pill",
                                                            onclick: move |_| on_show_dashboard.call(()),
                                                            span { "Về Dashboard" }
                                                            span { class: "pill_icon", "↗" }
                                                        }
                                                    }
                                                }
                                            },
                                            SaveNotice::Error(message) => rsx! {
                                                div { class: "error_box",
                                                    div { class: "status_copy",
                                                        "Đã extract xong nhưng lưu SQLite thất bại. {message}"
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card_shell create_result_shell",
                            div { class: "card create_result_card",
                                div { class: "card_header",
                                    div { class: "card_title", "Vocabulary Preview" }
                                    div { class: "card_hint",
                                        if let Some(preview) = result() {
                                            "{preview.vocabulary.len()} entries"
                                        } else {
                                            "Awaiting input"
                                        }
                                    }
                                }

                                if let Some(preview) = result() {
                                    div { class: "result_meta",
                                        div { class: "result_title", "{preview.title}" }
                                        div { class: "result_subline",
                                            span { "{preview.source}" }
                                            span { "·" }
                                            span { "{preview.sentence_count} sentences" }
                                        }
                                    }

                                    div { class: "vocab_list",
                                        for entry in preview.vocabulary.iter() {
                                            div { class: "vocab_row",
                                                div { class: "vocab_lemma", "{entry.lemma}" }
                                                div { class: "vocab_context", "{entry.context}" }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "empty_state",
                                        div { class: "empty_title", "No extraction yet" }
                                        div { class: "empty_copy",
                                            "Run a URL scrape or parse a local file to populate this panel with real lemmas and contexts from the Rust NLP pipeline."
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

async fn process_from_url(url: &str) -> anyhow::Result<CreateDeckResult> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        anyhow::bail!("URL không được để trống.");
    }

    let article = url_scraper::scrape_url(trimmed).await?;
    build_preview_and_save(article, "url")
}

fn process_from_file(path: &str) -> anyhow::Result<CreateDeckResult> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Đường dẫn file không được để trống.");
    }

    let article = file_parser::parse_file(Path::new(trimmed))?;
    build_preview_and_save(article, "file")
}

fn build_preview_and_save(article: ArticleContent, source_type: &str) -> anyhow::Result<CreateDeckResult> {
    let preview = build_preview(&article)?;
    match persist_deck(source_type, &article, &preview.vocabulary) {
        Ok(deck_id) => Ok(CreateDeckResult::Saved { preview, deck_id }),
        Err(err) => Ok(CreateDeckResult::SaveFailed {
            preview,
            message: format!("{err:#}"),
        }),
    }
}

fn build_preview(article: &ArticleContent) -> anyhow::Result<DeckPreview> {
    let source = article.url.clone();
    let sentence_count = article.sentences.len();
    let title = article.title.clone();
    let nlp = LexiFlashNlp::new()?;
    let vocabulary = nlp.process_article(&article.sentences);

    Ok(DeckPreview {
        title,
        source,
        sentence_count,
        vocabulary,
    })
}

fn persist_deck(
    source_type: &str,
    article: &ArticleContent,
    entries: &[VocabularyEntry],
) -> anyhow::Result<i64> {
    let db_path = db::default_db_path()?;
    let conn = db::init_db(&db_path)?;
    db::save_deck(
        &conn,
        &article.title,
        source_type,
        &article.url,
        article.sentences.len(),
        entries,
    )
}
