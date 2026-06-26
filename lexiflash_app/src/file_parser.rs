use anyhow::{bail, Context, Result};
use office_oxide::Document;
use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::PdfDocument;
use std::fs;
use std::path::{Path, PathBuf};

use crate::article_content::ArticleContent;
use crate::text_utils::{normalize_whitespace, split_sentences};

pub fn parse_file(path: &Path) -> Result<ArticleContent> {
    let normalized = ensure_local_file(path)?;
    let ext = normalized
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    let raw_text = match ext.as_str() {
        "txt" => read_txt_file(&normalized)?,
        "docx" => read_office_file(&normalized, "DOCX")?,
        "pptx" => read_office_file(&normalized, "PPTX")?,
        "pdf" => read_pdf_file(&normalized)?,
        _ => bail!(
            "unsupported file format: .{}. Supported: .txt, .docx, .pptx, .pdf",
            ext
        ),
    };

    let normalized_text = normalize_whitespace(&raw_text);
    let sentences = split_sentences(&normalized_text);
    if sentences.is_empty() {
        bail!("no sentences could be extracted from '{}'", normalized.display());
    }

    let title = normalized
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("untitled")
        .to_string();

    Ok(ArticleContent {
        url: normalized.display().to_string(),
        title,
        sentences,
    })
}

fn ensure_local_file(path: &Path) -> Result<PathBuf> {
    let normalized = path
        .canonicalize()
        .with_context(|| format!("input file not found or unreadable: '{}'", path.display()))?;

    if !normalized.is_file() {
        bail!("input path is not a file: '{}'", normalized.display());
    }

    Ok(normalized)
}

fn read_txt_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("could not read UTF-8 text file '{}'", path.display()))
}

fn read_office_file(path: &Path, kind: &str) -> Result<String> {
    let doc = Document::open(path)
        .with_context(|| format!("could not open {kind} file '{}'", path.display()))?;
    Ok(doc.plain_text())
}

fn read_pdf_file(path: &Path) -> Result<String> {
    let doc = PdfDocument::open(path)
        .with_context(|| format!("could not open PDF file '{}'", path.display()))?;
    let options = ConversionOptions::default();
    doc.to_plain_text_all(&options)
        .with_context(|| format!("failed to extract text from PDF '{}'", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_txt_file_into_sentences() {
        let path = std::env::temp_dir().join("lexiflash_file_parser_test.txt");
        fs::write(
            &path,
            "Reliable extraction matters. Study tools should stay calm and clear.",
        )
        .unwrap();

        let article = parse_file(&path).unwrap();
        assert_eq!(article.title, "lexiflash_file_parser_test");
        assert_eq!(article.sentences.len(), 2);

        let _ = fs::remove_file(path);
    }
}
