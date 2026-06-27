#[path = "../article_content.rs"]
mod article_content;
#[path = "../text_utils.rs"]
mod text_utils;
#[path = "../url_scraper.rs"]
mod url_scraper;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.bbc.com/news/articles/c70vqwengxno";
    let article = url_scraper::scrape_url(url).await?;

    println!("TITLE: {}", article.title);
    println!("SENTENCE_COUNT: {}", article.sentences.len());
    for sentence in article.sentences.iter().take(5) {
        println!("- {}", sentence);
    }

    Ok(())
}
