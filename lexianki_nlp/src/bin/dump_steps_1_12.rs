use anyhow::Result;
use lexianki_nlp::LexiankiNlp;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Input {
    sentences: Vec<String>,
}

fn main() -> Result<()> {
    let input: Input = serde_json::from_reader(std::io::stdin())?;
    let nlp = LexiankiNlp::new()?;
    let output = nlp.process_article(&input.sentences);
    serde_json::to_writer_pretty(std::io::stdout(), &output)?;
    Ok(())
}

