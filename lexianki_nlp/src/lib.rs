use anyhow::Result;
use nlprule::{tokenizer_filename, Tokenizer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedToken {
    pub token: String,
    pub pos: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WordnetPos {
    Noun,
    Verb,
    Adj,
    Adv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LemmatizedToken {
    pub token: String,
    pub pos: String,
    pub wordnet_pos: Option<WordnetPos>,
    pub lemma: String,
}

pub struct LexiankiNlp {
    tokenizer: Tokenizer,
}

impl LexiankiNlp {
    pub fn new() -> Result<Self> {
        let mut tokenizer_bytes: &[u8] = include_bytes!(concat!(
            env!("OUT_DIR"),
            "/",
            tokenizer_filename!("en")
        ));
        let tokenizer = Tokenizer::from_reader(&mut tokenizer_bytes)?;
        Ok(Self { tokenizer })
    }

    pub fn process_sentence_steps_1_4(&self, sentence: &str) -> Vec<TaggedToken> {
        let mut out = Vec::new();

        for sent in self.tokenizer.pipe(sentence) {
            let sentence_start_tokens: HashSet<String> = sent
                .tokens()
                .first()
                .map(|token| HashSet::from([token.word().as_str().to_lowercase()]))
                .unwrap_or_default();

            for token in sent.tokens() {
                let token_text = token.word().as_str();
                let primary = match token.word().tags().first() {
                    Some(tag) => tag,
                    None => continue,
                };

                let label = coarse_pos(primary.pos().as_str());
                if matches!(label, "NNP" | "NNPS") {
                    continue;
                }

                let word_lower = token_text.to_lowercase();

                if token_text
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase())
                    && !sentence_start_tokens.contains(&word_lower)
                {
                    continue;
                }

                if !is_valid_word(&word_lower) {
                    continue;
                }

                out.push(TaggedToken {
                    token: word_lower,
                    pos: label.to_string(),
                });
            }
        }

        out
    }

    pub fn process_sentence_steps_1_6(&self, sentence: &str) -> Vec<LemmatizedToken> {
        let mut out = Vec::new();

        for sent in self.tokenizer.pipe(sentence) {
            let sentence_start_tokens: HashSet<String> = sent
                .tokens()
                .first()
                .map(|token| HashSet::from([token.word().as_str().to_lowercase()]))
                .unwrap_or_default();

            for token in sent.tokens() {
                let token_text = token.word().as_str();
                let primary = match token.word().tags().first() {
                    Some(tag) => tag,
                    None => continue,
                };

                let label = coarse_pos(primary.pos().as_str());
                if matches!(label, "NNP" | "NNPS") {
                    continue;
                }

                let word_lower = token_text.to_lowercase();

                if token_text
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase())
                    && !sentence_start_tokens.contains(&word_lower)
                {
                    continue;
                }

                if !is_valid_word(&word_lower) {
                    continue;
                }

                let wn_pos = map_to_wordnet_pos(label);
                let lemma = primary.lemma().as_str().to_lowercase();

                out.push(LemmatizedToken {
                    token: word_lower,
                    pos: label.to_string(),
                    wordnet_pos: wn_pos,
                    lemma,
                });
            }
        }

        out
    }
}

pub fn is_valid_word(word: &str) -> bool {
    let valid_re = Regex::new(r"^\p{L}+$").expect("regex must compile");
    word.chars().count() >= 5 && valid_re.is_match(word)
}

fn coarse_pos(label: &str) -> &str {
    label.split(':').next().unwrap_or(label)
}

fn map_to_wordnet_pos(tag: &str) -> Option<WordnetPos> {
    match tag.chars().next() {
        Some('J') => Some(WordnetPos::Adj),
        Some('V') => Some(WordnetPos::Verb),
        Some('N') => Some(WordnetPos::Noun),
        Some('R') => Some(WordnetPos::Adv),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_basic_sentence_as_expected() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "She was reading the largest books in various categories.";
        let tokens = nlp.process_sentence_steps_1_4(sentence);
        let words: Vec<String> = tokens.into_iter().map(|t| t.token).collect();
        assert_eq!(
            words,
            vec!["reading", "largest", "books", "various", "categories"]
        );
    }

    #[test]
    fn filters_proper_nouns() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "Robert and Sarah visited the beautiful city of Paris.";
        let tokens = nlp.process_sentence_steps_1_4(sentence);
        let words: HashSet<String> = tokens.into_iter().map(|t| t.token).collect();
        assert!(!words.contains("robert"));
        assert!(!words.contains("sarah"));
        assert!(!words.contains("paris"));
        assert!(words.contains("visited"));
        assert!(words.contains("beautiful"));
    }

    #[test]
    fn validates_min_length_and_letters_only() {
        assert!(!is_valid_word("test"));
        assert!(is_valid_word("tests"));
        assert!(!is_valid_word("hello!"));
        assert!(!is_valid_word("café"));
        assert!(is_valid_word("cafés"));
    }

    #[test]
    fn lemmatizes_basic_sentence() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "She was reading the largest books in various categories.";
        let tokens = nlp.process_sentence_steps_1_6(sentence);
        let lemmas: Vec<String> = tokens.into_iter().map(|t| t.lemma).collect();
        assert_eq!(lemmas, vec!["read", "large", "book", "various", "category"]);
    }

    #[test]
    fn documents_known_lemmatization_difference_vs_nltk() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "Researchers are analyzing multilingual datasets for robust tagging.";
        let tokens = nlp.process_sentence_steps_1_6(sentence);
        let datasets = tokens.iter().find(|t| t.token == "datasets").unwrap();
        assert_eq!(datasets.lemma, "dataset");

        let tagging = tokens.iter().find(|t| t.token == "tagging").unwrap();
        assert_eq!(tagging.lemma, "tag");
    }
}
