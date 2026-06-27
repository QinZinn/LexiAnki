#[path = "../article_content.rs"]
mod article_content;
#[path = "../file_parser.rs"]
mod file_parser;
#[path = "../text_utils.rs"]
mod text_utils;
#[path = "../url_scraper.rs"]
mod url_scraper;

use lexiflash_app::db;
use lexianki_nlp::LexiankiNlp;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "file".to_string());
    let source = args.next().unwrap_or_else(|| "examples/sample_article.txt".to_string());

    let article = match mode.as_str() {
        "url" => url_scraper::scrape_url(&source).await?,
        "file" => file_parser::parse_file(Path::new(&source))?,
        _ => {
            return Err(format!("unsupported mode '{mode}', expected 'file' or 'url'").into());
        }
    };

    let nlp = LexiankiNlp::new()?;
    let entries = nlp.process_article(&article.sentences);

    let db_path = db::default_db_path()?;
    let conn = db::init_db(&db_path)?;
    let deck_id = db::save_deck(
        &conn,
        &article.title,
        mode.as_str(),
        &article.url,
        article.sentences.len(),
        &entries,
    )?;

    let decks = db::list_decks(&conn)?;
    let snapshot = db::load_study_snapshot(&conn)?;

    println!("DB_PATH: {}", db_path.display());
    println!("SAVED_DECK_ID: {deck_id}");
    println!("TITLE: {}", article.title);
    println!("SENTENCE_COUNT: {}", article.sentences.len());
    println!("VOCAB_COUNT: {}", entries.len());
    println!("DECK_TOTAL: {}", decks.len());
    println!("LEARNED_TOTAL: {}", snapshot.learned_total);
    println!("DUE_TODAY: {}", snapshot.due_today);

    for deck in decks.iter().take(5) {
        println!(
            "- {} | {} | {} words | {} sentences",
            deck.id, deck.title, deck.vocabulary_count, deck.sentence_count
        );
    }

    Ok(())
}
