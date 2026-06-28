#[path = "../article_content.rs"]
mod article_content;
#[path = "../file_parser.rs"]
mod file_parser;
#[path = "../text_utils.rs"]
mod text_utils;

use lexiflash_nlp::LexiFlashNlp;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/sample_article.txt"));

    let article = file_parser::parse_file(&path)?;
    let nlp = LexiFlashNlp::new()?;
    let vocab = nlp.process_article(&article.sentences);

    println!("TITLE: {}", article.title);
    println!("SENTENCE_COUNT: {}", article.sentences.len());
    println!("VOCAB_COUNT: {}", vocab.len());
    for entry in vocab.iter().take(8) {
        println!("- {} => {}", entry.lemma, entry.context);
    }

    Ok(())
}

