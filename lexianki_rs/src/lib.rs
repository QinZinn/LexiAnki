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

fn find_token_match(sentence: &str, target_token: &str) -> Option<(usize, usize)> {
    if target_token.is_empty() {
        return None;
    }

    let pattern = format!(r"(?i){}", regex::escape(target_token));
    let token_re = Regex::new(&pattern).ok()?;

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

#[pymodule]
fn lexianki_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(is_valid_word, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_context, m)?)?;
    Ok(())
}

#[pyfunction]
fn truncate_context(sentence: &str, target_token: &str, max_length: usize) -> PyResult<String> {
    if max_length == 0 {
        return Ok(String::new());
    }

    let sentence_len = sentence.chars().count();
    if sentence_len <= max_length {
        return Ok(sentence.to_string());
    }

    if max_length <= 3 {
        return Ok("...".chars().take(max_length).collect());
    }

    let (match_start, match_end) = match find_token_match(sentence, target_token) {
        Some(m) => m,
        None => {
            let prefix = sentence.chars().take(max_length - 3).collect::<String>();
            return Ok(format!("{}...", prefix));
        }
    };

    let before = sentence.get(..match_start).unwrap_or("");
    let matched = sentence.get(match_start..match_end).unwrap_or("");

    let start_char = before.chars().count();
    let token_len = matched.chars().count();
    let end_char = start_char + token_len;

    if max_length <= token_len {
        return Ok(target_token.to_string());
    }

    let prefix_available = start_char;
    let suffix_available = sentence_len.saturating_sub(end_char);

    let budget_for_context = max_length.saturating_sub(token_len);
    let mut prefix_take = (budget_for_context / 2).min(prefix_available);
    let mut suffix_take = (budget_for_context - prefix_take).min(suffix_available);

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
        } else {
            if suffix_take > 0 {
                suffix_take -= 1;
            } else if prefix_take > 0 {
                prefix_take -= 1;
            }
        }
    }

    if total_len > max_length {
        return Ok(target_token.to_string());
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

    Ok(result)
}
