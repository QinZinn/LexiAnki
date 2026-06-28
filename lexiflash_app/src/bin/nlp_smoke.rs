use lexiflash_nlp::LexiFlashNlp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nlp = LexiFlashNlp::new()?;
    let sentences = vec![
        "Researchers are analyzing multilingual datasets for robust tagging.".to_string(),
        "A clever parser should ignore malformed tokens gracefully.".to_string(),
    ];
    let out = nlp.process_article(&sentences);
    println!("{out:#?}");
    Ok(())
}
