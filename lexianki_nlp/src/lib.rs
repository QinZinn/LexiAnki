use anyhow::{Context, Result};
use nlprule::{tokenizer_filename, Tokenizer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use wordnet_db::{LoadMode, WordNet};
use wordnet_types::{Pos, SynsetId};
use zip::ZipArchive;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatedToken {
    pub token: String,
    pub pos: String,
    pub wordnet_pos: Option<WordnetPos>,
    pub lemma: String,
    pub was_capitalized: bool,
    pub wordnet_gate_checked: bool,
    pub wordnet_lexname: Option<String>,
}

struct WordnetResources {
    lexnames: HashMap<u8, String>,
    database: WordNet,
}

pub struct LexiankiNlp {
    tokenizer: Tokenizer,
    wordnet: WordnetResources,
}

struct PreparedToken {
    lower_token: String,
    pos: String,
    wordnet_pos: Option<WordnetPos>,
    lemma: String,
    was_capitalized: bool,
    is_sentence_start: bool,
}

impl LexiankiNlp {
    pub fn new() -> Result<Self> {
        let mut tokenizer_bytes: &[u8] = include_bytes!(concat!(
            env!("OUT_DIR"),
            "/",
            tokenizer_filename!("en")
        ));
        let tokenizer = Tokenizer::from_reader(&mut tokenizer_bytes)?;
        let wordnet = load_wordnet_resources()?;
        Ok(Self { tokenizer, wordnet })
    }

    pub fn process_sentence_steps_1_4(&self, sentence: &str) -> Vec<TaggedToken> {
        self.prepare_tokens(sentence)
            .into_iter()
            .map(|token| TaggedToken {
                token: token.lower_token,
                pos: token.pos,
            })
            .collect()
    }

    pub fn process_sentence_steps_1_6(&self, sentence: &str) -> Vec<LemmatizedToken> {
        self.prepare_tokens(sentence)
            .into_iter()
            .map(|token| LemmatizedToken {
                token: token.lower_token,
                pos: token.pos,
                wordnet_pos: token.wordnet_pos,
                lemma: token.lemma,
            })
            .collect()
    }

    pub fn process_sentence_steps_1_8(&self, sentence: &str) -> Vec<GatedToken> {
        self.prepare_tokens(sentence)
            .into_iter()
            .filter_map(|token| self.apply_steps_7_8(token))
            .collect()
    }

    fn prepare_tokens(&self, sentence: &str) -> Vec<PreparedToken> {
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
                if !is_valid_word(&word_lower) {
                    continue;
                }

                let lemma = primary.lemma().as_str().to_lowercase();

                out.push(PreparedToken {
                    lower_token: word_lower.clone(),
                    pos: label.to_string(),
                    wordnet_pos: map_to_wordnet_pos(label),
                    lemma,
                    was_capitalized: token_text.chars().next().is_some_and(|c| c.is_uppercase()),
                    is_sentence_start: sentence_start_tokens.contains(&word_lower),
                });
            }
        }

        out
    }

    fn apply_steps_7_8(&self, token: PreparedToken) -> Option<GatedToken> {
        if token.was_capitalized && !token.is_sentence_start {
            return None;
        }

        let mut wordnet_gate_checked = false;
        let mut wordnet_lexname = None;

        if token.was_capitalized {
            wordnet_gate_checked = true;
            if let Some(synset_id) = first_synset_id_for_lemma(&self.wordnet.database, &token.lemma) {
                if let Some(synset) = self.wordnet.database.get_synset(synset_id) {
                    if let Some(name) = self.wordnet.lexnames.get(&synset.lex_filenum) {
                        wordnet_lexname = Some(name.clone());
                        if is_proper_lexname(name) {
                            return None;
                        }
                    }
                }
            }
        }

        Some(GatedToken {
            token: token.lower_token,
            pos: token.pos,
            wordnet_pos: token.wordnet_pos,
            lemma: token.lemma,
            was_capitalized: token.was_capitalized,
            wordnet_gate_checked,
            wordnet_lexname,
        })
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

fn find_wordnet_zip() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("WORDNET_ZIP_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var("HOME").context("HOME must be set to locate nltk_data")?;
    let candidates = [
        PathBuf::from(&home).join("nltk_data/corpora/wordnet.zip"),
        PathBuf::from(&home).join(".local/share/nltk_data/corpora/wordnet.zip"),
        PathBuf::from("/usr/share/nltk_data/corpora/wordnet.zip"),
        PathBuf::from("/usr/local/share/nltk_data/corpora/wordnet.zip"),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("could not find wordnet.zip; set WORDNET_ZIP_PATH to NLTK wordnet.zip")
}

fn wordnet_cache_dir() -> Result<PathBuf> {
    if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(cache_home).join("lexianki_nlp/wordnet_extracted"));
    }

    let home = std::env::var("HOME").context("HOME must be set to derive cache directory")?;
    Ok(PathBuf::from(home).join(".cache/lexianki_nlp/wordnet_extracted"))
}

fn ensure_extracted_wordnet_dir(zip_path: &Path) -> Result<PathBuf> {
    let out_dir = wordnet_cache_dir()?;
    fs::create_dir_all(&out_dir)?;

    let required = [
        "data.noun",
        "data.verb",
        "data.adj",
        "data.adv",
        "index.noun",
        "index.verb",
        "index.adj",
        "index.adv",
        "lexnames",
    ];

    if required.iter().all(|name| out_dir.join(name).exists()) {
        return Ok(out_dir);
    }

    for name in required {
        let dest = out_dir.join(name);
        if dest.exists() {
            continue;
        }

        let entry_name = format!("wordnet/{}", name);
        let file = File::open(zip_path)
            .with_context(|| format!("failed to open wordnet zip '{}'", zip_path.display()))?;
        let mut zip = ZipArchive::new(file)
            .with_context(|| format!("failed to read zip '{}'", zip_path.display()))?;
        let mut entry = zip
            .by_name(&entry_name)
            .with_context(|| format!("missing '{}' in '{}'", entry_name, zip_path.display()))?;

        let mut out = File::create(&dest)
            .with_context(|| format!("failed to create '{}'", dest.display()))?;
        std::io::copy(&mut entry, &mut out)?;
        out.flush()?;
    }

    Ok(out_dir)
}

fn load_lexnames(wordnet_dir: &Path) -> Result<HashMap<u8, String>> {
    let mut content = String::new();
    File::open(wordnet_dir.join("lexnames"))
        .context("failed to open lexnames")?
        .read_to_string(&mut content)?;

    let mut map = HashMap::new();
    for line in content.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut parts = line.split_whitespace();
        let id = match parts.next() {
            Some(value) => value,
            None => continue,
        };
        let name = match parts.next() {
            Some(value) => value,
            None => continue,
        };
        if let Ok(parsed) = id.parse::<u8>() {
            map.insert(parsed, name.to_string());
        }
    }

    Ok(map)
}

fn load_wordnet_resources() -> Result<WordnetResources> {
    let wordnet_zip = find_wordnet_zip()?;
    let wordnet_dir = ensure_extracted_wordnet_dir(&wordnet_zip)?;
    let lexnames = load_lexnames(&wordnet_dir)?;
    let database = WordNet::load_with_mode(&wordnet_dir, LoadMode::Mmap)?;
    Ok(WordnetResources { lexnames, database })
}

fn first_synset_id_for_lemma(wordnet: &WordNet, lemma: &str) -> Option<SynsetId> {
    for pos in [Pos::Noun, Pos::Verb, Pos::Adj, Pos::Adv] {
        if let Some(id) = wordnet.synsets_for_lemma(pos, lemma).first() {
            return Some(*id);
        }
    }
    None
}

fn is_proper_lexname(lexname: &str) -> bool {
    matches!(
        lexname,
        "noun.person" | "noun.location" | "noun.group" | "noun.object"
    )
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

    #[test]
    fn heuristic_filters_mid_sentence_capitalized_tokens() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "Our American guide described Parisian architecture to curious visitors.";
        let tokens = nlp.process_sentence_steps_1_8(sentence);
        let words: HashSet<String> = tokens.into_iter().map(|t| t.token).collect();
        assert!(!words.contains("american"));
        assert!(!words.contains("parisian"));
        assert!(words.contains("described"));
        assert!(words.contains("architecture"));
    }

    #[test]
    fn wordnet_gate_filters_sentence_start_american() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "American researchers visited Boston during Vietnamese cultural events.";
        let tokens = nlp.process_sentence_steps_1_8(sentence);
        let words: HashSet<String> = tokens.iter().map(|t| t.token.clone()).collect();
        assert!(!words.contains("american"));
        assert!(!words.contains("vietnamese"));
        assert!(words.contains("researchers"));
        assert!(words.contains("visited"));
    }
}
