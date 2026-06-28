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

    let mut outputs = Vec::new();
    for sentence in input.sentences {
        let tokens = nlp.process_sentence_steps_1_4(&sentence);
        outputs.push(tokens);
    }

    serde_json::to_writer_pretty(std::io::stdout(), &outputs)?;
    Ok(())
}

