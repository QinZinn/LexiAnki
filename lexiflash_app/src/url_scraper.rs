use anyhow::{bail, Context, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

use crate::article_content::ArticleContent;
use crate::text_utils::split_sentences;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub async fn scrape_url(url: &str) -> Result<ArticleContent> {
    let body = fetch_with_retry(url, 3).await?;
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

async fn fetch_with_retry(url: &str, attempts: usize) -> Result<String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .context("failed to build HTTP client")?;

    let mut last_error = None;

    for attempt in 1..=attempts {
        match client.get(url).send().await {
            Ok(response) => {
                let response = response
                    .error_for_status()
                    .with_context(|| format!("HTTP error while fetching {url}"))?;
                return response
                    .text()
                    .await
                    .with_context(|| format!("failed to read response body from {url}"));
            }
            Err(err) => {
                last_error = Some(err);
                if attempt < attempts {
                    let delay_ms = 400_u64 * attempt as u64;
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[tokio::test]
    async fn scrape_url_runs_inside_async_runtime() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);

            let body = "<html><head><title>Async Article</title></head><body><article><p>Reliable sentence extraction matters for study tools. Calm interfaces should stay responsive when users fetch a new article. The parser should return meaningful vocabulary without crashing the desktop session.</p></article></body></html>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        });

        let article = scrape_url(&format!("http://{address}")).await.unwrap();
        handle.join().unwrap();

        assert_eq!(article.title, "Async Article");
        assert_eq!(article.sentences.len(), 3);
        assert!(article
            .sentences
            .iter()
            .any(|sentence| sentence.contains("desktop session")));
    }
}
