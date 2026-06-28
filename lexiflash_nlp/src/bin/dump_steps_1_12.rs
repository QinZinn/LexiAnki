use anyhow::Result;
use lexiflash_nlp::LexiFlashNlp;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Input {
    sentences: Vec<String>,
}

fn main() -> Result<()> {
    let input: Input = serde_json::from_reader(std::io::stdin())?;
    let nlp = LexiFlashNlp::new()?;
    let output = nlp.process_article(&input.sentences);
    serde_json::to_writer_pretty(std::io::stdout(), &output)?;
    Ok(())
}

