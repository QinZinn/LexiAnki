pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn split_sentences(content: &str) -> Vec<String> {
    let normalized = normalize_whitespace(content);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = normalized.chars().collect();

    for (index, ch) in chars.iter().copied().enumerate() {
        current.push(ch);

        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }

        let next_non_ws = chars
            .iter()
            .skip(index + 1)
            .copied()
            .find(|candidate| !candidate.is_whitespace());

        let should_split = match next_non_ws {
            Some(next) => next.is_uppercase() || matches!(next, '"' | '\''),
            None => true,
        };

        if should_split {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                sentences.push(trimmed.to_string());
            }
            current.clear();
        }
    }

    let trailing = current.trim();
    if !trailing.is_empty() {
        sentences.push(trailing.to_string());
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sentences_on_terminal_punctuation() {
        let content = "Alpha rises quickly. Beta follows soon! \"Gamma\" remains stable?";
        let sentences = split_sentences(content);
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Alpha rises quickly.");
        assert_eq!(sentences[1], "Beta follows soon!");
        assert_eq!(sentences[2], "\"Gamma\" remains stable?");
    }
}

