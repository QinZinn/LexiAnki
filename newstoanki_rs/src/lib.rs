use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use regex::Regex;

use lazy_static::lazy_static;

lazy_static! {
    static ref VALID_WORD_RE: Regex = Regex::new(r"^[a-zA-ZÀ-ÿ]+$").unwrap();
}

#[pyfunction]
fn is_valid_word(word: &str) -> PyResult<bool> {
    if word.chars().count() < 5 {
        return Ok(false);
    }
    Ok(VALID_WORD_RE.is_match(word))
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

#[pymodule]
fn newstoanki_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(is_valid_word, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_context, m)?)?;
    Ok(())
}

#[pyfunction]
fn truncate_context(sentence: &str, target_token: &str, max_length: usize) -> PyResult<String> {
    let sentence_len = sentence.chars().count();
    if sentence_len <= max_length {
        return Ok(sentence.to_string());
    }

    if max_length <= 3 {
        return Ok("...".chars().take(max_length).collect());
    }

    let pattern = format!(r"(?i)\b{}\b", regex::escape(target_token));
    let token_re = Regex::new(&pattern).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mat = match token_re.find(sentence) {
        Some(m) => m,
        None => {
            let prefix = sentence.chars().take(max_length - 3).collect::<String>();
            return Ok(format!("{}...", prefix));
        }
    };

    let before = &sentence[..mat.start()];
    let matched = &sentence[mat.start()..mat.end()];

    let start_char = before.chars().count();
    let token_len = matched.chars().count();
    let end_char = start_char + token_len;

    if max_length <= token_len {
        return Ok(matched.chars().take(max_length).collect());
    }

    let remaining = max_length.saturating_sub(token_len).saturating_sub(6);
    let prefix_len = remaining / 2;
    let suffix_len = remaining - prefix_len;

    let mut new_start = start_char.saturating_sub(prefix_len);
    let mut new_end = (end_char + suffix_len).min(sentence_len);

    if new_start == 0 {
        new_end = sentence_len.min(max_length - 3);
    } else if new_end == sentence_len {
        new_start = sentence_len.saturating_sub(max_length - 3);
        new_end = sentence_len;
    }

    if new_end <= new_start {
        let prefix = sentence.chars().take(max_length - 3).collect::<String>();
        return Ok(format!("{}...", prefix));
    }

    let mut result = substring_by_char_range(sentence, new_start, new_end);

    if new_start > 0 {
        result = format!("...{}", result);
    }
    if new_end < sentence_len {
        result = format!("{}...", result);
    }

    if result.chars().count() > max_length {
        result = result.chars().take(max_length).collect();
    }

    Ok(result)
}
