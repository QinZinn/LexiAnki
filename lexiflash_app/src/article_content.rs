#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleContent {
    pub url: String,
    pub title: String,
    pub sentences: Vec<String>,
}
