use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::thread;
use std::time::Duration;

use crate::article_content::ArticleContent;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub fn scrape_url(url: &str) -> Result<ArticleContent> {
    let body = fetch_with_retry(url, 3)?;
    let document = Html::parse_document(&body);
    let title = extract_title(&document);
    let content = extract_content(url, &document);
    let sentences = split_sentences(&content);

    if sentences.is_empty() {
        bail!("no article sentences could be extracted from {url}");
    }

    Ok(ArticleContent {
        url: url.to_string(),
        title,
        sentences,
    })
}

fn fetch_with_retry(url: &str, attempts: usize) -> Result<String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;

    for attempt in 1..=attempts {
        match client.get(url).send() {
            Ok(response) => {
                let response = response
                    .error_for_status()
                    .with_context(|| format!("HTTP error while fetching {url}"))?;
                return response
                    .text()
                    .with_context(|| format!("failed to read response body from {url}"));
            }
            Err(err) => {
                last_error = Some(err);
                if attempt < attempts {
                    let delay_ms = 400_u64 * attempt as u64;
                    thread::sleep(Duration::from_millis(delay_ms));
                }
            }
        }
    }

    Err(last_error
        .context("request failed without a captured error")?
        .into())
}

fn extract_title(document: &Html) -> String {
    let selectors = ["h1", "title"];
    for selector in selectors {
        if let Some(text) = first_text(document, selector) {
            return text;
        }
    }
    "Unknown Title".to_string()
}

fn extract_content(url: &str, document: &Html) -> String {
    let lowered = url.to_ascii_lowercase();

    let selectors = if lowered.contains("bbc.com") || lowered.contains("bbc.co.uk") {
        vec!["article p", "main p", "p"]
    } else if lowered.contains("vnexpress.net") {
        vec!["p.description", "p.Normal", "article p", "p"]
    } else {
        vec!["article p", "main p", "p"]
    };

    for selector in selectors {
        let joined = joined_text(document, selector);
        if joined.split_whitespace().count() >= 40 {
            return joined;
        }
    }

    String::new()
}

fn first_text(document: &Html, css: &str) -> Option<String> {
    let selector = Selector::parse(css).ok()?;
    document
        .select(&selector)
        .next()
        .map(extract_node_text)
        .filter(|text| !text.is_empty())
}

fn joined_text(document: &Html, css: &str) -> String {
    let selector = match Selector::parse(css) {
        Ok(value) => value,
        Err(_) => return String::new(),
    };

    let fragments: Vec<String> = document
        .select(&selector)
        .map(extract_node_text)
        .filter(|text| !text.is_empty())
        .collect();

    fragments.join(" ")
}

fn extract_node_text(node: scraper::ElementRef<'_>) -> String {
    node.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn split_sentences(content: &str) -> Vec<String> {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
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
