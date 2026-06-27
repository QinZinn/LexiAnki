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
    pub original_token: String,
    pub was_capitalized: bool,
    pub wordnet_gate_checked: bool,
    pub wordnet_lexname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyEntry {
    pub lemma: String,
    pub context: String,
    pub original_token: String,
    pub wordnet_pos: Option<WordnetPos>,
}

struct WordnetResources {
    lexnames: HashMap<u8, String>,
    database: WordNet,
}

pub struct LexiankiNlp {
    tokenizer: Tokenizer,
    wordnet: WordnetResources,
    stop_words: HashSet<String>,
    known_words: HashSet<String>,
}

struct PreparedToken {
    original_token: String,
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
        let stop_words = load_stopwords_english()?;
        let known_words = load_known_words(&default_known_words_path()?)?;
        Ok(Self {
            tokenizer,
            wordnet,
            stop_words,
            known_words,
        })
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

    pub fn process_sentence_full(&self, sentence: &str) -> Vec<VocabularyEntry> {
        let sentences = vec![sentence.to_string()];
        self.process_article(&sentences)
    }

    pub fn process_article(&self, sentences: &[String]) -> Vec<VocabularyEntry> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        for sentence in sentences {
            for token in self.prepare_tokens(sentence) {
                let gated = match self.apply_steps_7_8(token) {
                    Some(value) => value,
                    None => continue,
                };

                if !is_valid_word(&gated.lemma) {
                    continue;
                }

                if self.stop_words.contains(&gated.lemma) {
                    continue;
                }

                if self.known_words.contains(&gated.lemma) {
                    continue;
                }

                if !seen.insert(gated.lemma.clone()) {
                    continue;
                }

                let context = truncate_context(sentence, &gated.original_token, 150);
                out.push(VocabularyEntry {
                    lemma: gated.lemma,
                    context,
                    original_token: gated.original_token,
                    wordnet_pos: gated.wordnet_pos,
                });
            }
        }

        out
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
                    original_token: token_text.to_string(),
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
            original_token: token.original_token,
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
    lexname == "noun.person"
        || lexname == "noun.location"
        || lexname == "noun.group"
        || lexname == "noun.object"
}

fn default_known_words_path() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .context("CARGO_MANIFEST_DIR must have a parent to locate repo root")?;
    Ok(repo_root.join("known_words.txt"))
}

fn load_known_words(path: &Path) -> Result<HashSet<String>> {
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(err) => return Err(err).with_context(|| format!("failed to read '{}'", path.display())),
    };

    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_lowercase())
        .collect())
}

fn find_stopwords_english_file() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("STOPWORDS_ENGLISH_PATH") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(nltk_data) = std::env::var("NLTK_DATA") {
        for root in nltk_data.split(':').filter(|part| !part.is_empty()) {
            let candidate = PathBuf::from(root).join("corpora/stopwords/english");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    let home = std::env::var("HOME").context("HOME must be set to locate nltk_data")?;
    let candidates = [
        PathBuf::from(&home).join("nltk_data/corpora/stopwords/english"),
        PathBuf::from(&home).join(".local/share/nltk_data/corpora/stopwords/english"),
        PathBuf::from("/usr/share/nltk_data/corpora/stopwords/english"),
        PathBuf::from("/usr/local/share/nltk_data/corpora/stopwords/english"),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "could not find NLTK stopwords corpus file; set STOPWORDS_ENGLISH_PATH or download NLTK stopwords"
    )
}

fn load_stopwords_english() -> Result<HashSet<String>> {
    let path = find_stopwords_english_file()?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read stopwords from '{}'", path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_lowercase())
        .collect())
}

fn byte_index_for_char_index(s: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    match s.char_indices().nth(char_index) {
        Some((byte_index, _)) => byte_index,
        None => s.len(),
    }
}

fn substring_by_char_range(s: &str, start_char: usize, end_char: usize) -> String {
    let start_byte = byte_index_for_char_index(s, start_char);
    let end_byte = byte_index_for_char_index(s, end_char);
    s.get(start_byte..end_byte).unwrap_or("").to_string()
}

// This Rust-native copy is intentional: `lexiflash_app` and the new desktop
// pipeline use `lexianki_nlp`, while the legacy Python CLI still calls the PyO3
// implementation in `lexianki_rs`. Keep both implementations behaviorally
// aligned when the truncation algorithm changes.
fn find_token_match(sentence: &str, target_token: &str) -> Option<(usize, usize)> {
    if target_token.is_empty() {
        return None;
    }

    if target_token.chars().count() < 2 {
        return None;
    }

    let escaped = regex::escape(target_token);
    let pattern = format!(r"(?i){}", escaped);
    let token_re = regex::RegexBuilder::new(&pattern)
        .size_limit(1 << 18)
        .build()
        .ok()?;

    for mat in token_re.find_iter(sentence) {
        let prev_slice = sentence.get(..mat.start()).unwrap_or("");
        let prev_is_alpha = prev_slice
            .chars()
            .rev()
            .next()
            .is_some_and(|c| c.is_alphabetic());
        if prev_is_alpha {
            continue;
        }

        let next_slice = sentence.get(mat.end()..).unwrap_or("");
        let next_is_alpha = next_slice
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic());
        if next_is_alpha {
            continue;
        }

        return Some((mat.start(), mat.end()));
    }

    None
}

fn truncate_context(sentence: &str, target_token: &str, max_length: usize) -> String {
    if max_length == 0 {
        return String::new();
    }

    let sentence_len = sentence.chars().count();
    if sentence_len <= max_length {
        return sentence.to_string();
    }

    if max_length <= 3 {
        return "...".chars().take(max_length).collect();
    }

    let (match_start, match_end) = match find_token_match(sentence, target_token) {
        Some(value) => value,
        None => {
            let prefix = sentence.chars().take(max_length - 3).collect::<String>();
            return format!("{}...", prefix);
        }
    };

    let before = sentence.get(..match_start).unwrap_or("");
    let matched = sentence.get(match_start..match_end).unwrap_or("");

    let start_char = before.chars().count();
    let token_len = matched.chars().count();
    let end_char = start_char + token_len;

    if max_length <= token_len {
        return target_token.to_string();
    }

    let prefix_available = start_char;
    let suffix_available = sentence_len.saturating_sub(end_char);

    let budget_for_context = max_length.saturating_sub(token_len);
    let left_ellipsis = if prefix_available > 0 { 3 } else { 0 };
    let right_ellipsis = if suffix_available > 0 { 3 } else { 0 };
    let adjusted_budget = budget_for_context.saturating_sub(left_ellipsis + right_ellipsis);
    let mut prefix_take = (adjusted_budget / 2).min(prefix_available);
    let mut suffix_take = (adjusted_budget - prefix_take).min(suffix_available);

    let mut left_needed;
    let mut right_needed;
    let mut total_len;

    loop {
        left_needed = prefix_take < prefix_available;
        right_needed = suffix_take < suffix_available;
        total_len = token_len
            + prefix_take
            + suffix_take
            + if left_needed { 3 } else { 0 }
            + if right_needed { 3 } else { 0 };

        if total_len <= max_length {
            break;
        }

        if prefix_take == 0 && suffix_take == 0 {
            break;
        }

        if prefix_take > suffix_take {
            prefix_take -= 1;
        } else if suffix_take > 0 {
            suffix_take -= 1;
        } else if prefix_take > 0 {
            prefix_take -= 1;
        }
    }

    if total_len > max_length {
        return target_token.to_string();
    }

    let new_start = start_char.saturating_sub(prefix_take);
    let new_end = (end_char + suffix_take).min(sentence_len);

    let mut result = substring_by_char_range(sentence, new_start, new_end);

    if left_needed {
        result = format!("...{}", result);
    }
    if right_needed {
        result = format!("{}...", result);
    }

    if result.chars().count() > max_length {
        result = result.chars().take(max_length).collect();
    }

    result
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

    #[test]
    fn post_lemmatization_validation_filters_short_lemmas() {
        let nlp = LexiankiNlp::new().unwrap();
        let sentence = "Researchers are analyzing multilingual datasets for robust tagging.";
        let entries = nlp.process_sentence_full(sentence);
        assert!(!entries.iter().any(|entry| entry.lemma.chars().count() < 5));
        assert!(!entries.iter().any(|entry| entry.lemma == "tag"));
    }
}
