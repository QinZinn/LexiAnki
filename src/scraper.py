import logging
import requests
from bs4 import BeautifulSoup
import nltk
from nltk.tokenize import sent_tokenize, word_tokenize
from urllib.parse import urlparse
from fake_useragent import UserAgent
from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception_type

# Setup logger for this module
logger = logging.getLogger(__name__)
_NLTK_SETUP_DONE = False

def setup_nltk():
    """
    Ensures that required NLTK datasets/models are downloaded.
    We need 'punkt' or 'punkt_tab' for sentence and word tokenization.
    """
    global _NLTK_SETUP_DONE
    if _NLTK_SETUP_DONE:
        return

    try:
        nltk.data.find('tokenizers/punkt_tab')
    except (LookupError, OSError):
        logger.info("Downloading NLTK punkt_tab dataset...")
        nltk.download('punkt_tab', quiet=True)
        # Fallback to punkt if punkt_tab isn't fully resolving in older versions
        nltk.download('punkt', quiet=True)

    _NLTK_SETUP_DONE = True

class BaseScraper:
    """
    Base class for news scrapers. 
    Implements the orchestration logic for fetching and tokenizing articles.
    """
    def __init__(self):
        try:
            self._ua = UserAgent()
        except Exception as exc:
            logger.warning(f"Failed to initialize fake user agent provider: {exc}")
            self._ua = None

    @staticmethod
    def _empty_article(url: str, title: str = "Unknown Title") -> dict:
        return {
            "url": url,
            "title": title,
            "data": [],
        }

    @staticmethod
    def _extract_text(tag, fallback: str) -> str:
        if tag is None:
            return fallback
        text = tag.get_text(strip=True)
        return text or fallback

    @staticmethod
    def _join_paragraphs(paragraphs) -> str:
        return " ".join(
            paragraph.get_text(strip=True)
            for paragraph in paragraphs
            if paragraph.get_text(strip=True)
        )

    @retry(
        stop=stop_after_attempt(3),
        wait=wait_exponential(multiplier=1, min=2, max=10),
        retry=retry_if_exception_type(requests.RequestException),
        reraise=True,
        before_sleep=lambda retry_state: logging.getLogger(__name__).info(
            f"Retrying fetch ({retry_state.attempt_number}/3)..."
        )
    )
    def _get_response(self, url: str) -> requests.Response:
        """
        Internal method to fetch URL with random User-Agent and retries.
        """
        user_agent = self._ua.random if self._ua else "Mozilla/5.0 (compatible; LexiFlash/1.0)"
        headers = {'User-Agent': user_agent}
        response = requests.get(url, headers=headers, timeout=10)
        response.raise_for_status()
        return response

    def fetch_article(self, url: str) -> dict:
        """
        Orchestrates the scraping process.
        
        Args:
            url (str): The URL of the news article to scrape.

        Returns:
            dict: Standardized article data structure. Failures return an empty article payload.
        """
        logger.info(f"Using {self.__class__.__name__} to fetch: {url}")
        try:
            response = self._get_response(url)
        except requests.RequestException as exc:
            logger.error(
                f"Network fetch failed for {url} with {self.__class__.__name__}: "
                f"{exc.__class__.__name__}: {exc}"
            )
            return self._empty_article(url)

        soup = BeautifulSoup(response.content, 'html.parser')
        
        # Site-specific extraction
        try:
            title, content = self.extract_title_and_content(soup)
        except (AttributeError, TypeError, ValueError) as exc:
            logger.error(
                f"HTML parsing failed for {url} with {self.__class__.__name__}: "
                f"{exc.__class__.__name__}: {exc}"
            )
            return self._empty_article(url)

        logger.info(f"Extracted title: {title}")
        
        if not content:
            logger.warning(
                f"No content extracted using {self.__class__.__name__}. "
                "The site might be blocking scrapers or using a different structure."
            )
            return self._empty_article(url, title)

        # Make sure tokenizers are available
        try:
            setup_nltk()
            sentences = sent_tokenize(content)
        except (LookupError, OSError, ValueError) as exc:
            logger.error(
                f"Sentence tokenization failed for {url}: "
                f"{exc.__class__.__name__}: {exc}"
            )
            return self._empty_article(url, title)

        logger.info(f"Extracted {len(sentences)} sentences.")

        # Prepare data structure
        data = []
        for sentence in sentences:
            try:
                words = word_tokenize(sentence)
            except (LookupError, OSError, ValueError) as exc:
                logger.warning(
                    f"Skipping sentence because tokenization failed for {url}: "
                    f"{exc.__class__.__name__}: {exc}"
                )
                continue
            data.append({
                "sentence": sentence,
                "words": words
            })

        return {
            'url': url,
            'title': title,
            'data': data
        }

    def extract_title_and_content(self, soup: BeautifulSoup) -> tuple[str, str]:
        """
        Abstract-like method to be implemented by child classes.
        
        Returns:
            tuple: (title, content_string)
        """
        raise NotImplementedError("Subclasses must implement extract_title_and_content")

class VnExpressScraper(BaseScraper):
    """
    Scraper specifically for VnExpress (English version).
    """
    def extract_title_and_content(self, soup: BeautifulSoup) -> tuple[str, str]:
        # Extract Title
        title_tag = soup.find('h1', class_='title-detail') or soup.find('title')
        title = self._extract_text(title_tag, "Unknown VnExpress Title")
        
        # Extract Content
        # VnExpress usually puts content in <p> tags with class 'Normal' or 'description'
        paragraphs = soup.find_all('p', class_='description')
        paragraphs += soup.find_all('p', class_='Normal')
        
        if not paragraphs:
            # Fallback to generic <p> if specific classes aren't found
            paragraphs = soup.find_all('p')
            
        content = self._join_paragraphs(paragraphs)
        return title, content

class BBCScraper(BaseScraper):
    """
    Skeleton scraper for BBC News.
    """
    def extract_title_and_content(self, soup: BeautifulSoup) -> tuple[str, str]:
        # Extract Title
        title_tag = soup.find('h1') or soup.find('title')
        title = self._extract_text(title_tag, "Unknown BBC Title")
        
        # BBC News often uses specific structures. This is a generic/skeleton approach.
        # Fallback to generic <article> then <p> tags.
        article = soup.find('article')
        if article:
            paragraphs = article.find_all('p')
        else:
            paragraphs = soup.find_all('p')
            
        content = self._join_paragraphs(paragraphs)
        return title, content

class GenericScraper(BaseScraper):
    """
    Fallback scraper for unknown domains.
    """
    def extract_title_and_content(self, soup: BeautifulSoup) -> tuple[str, str]:
        title_tag = soup.find('title')
        title = self._extract_text(title_tag, "Unknown Title")
        paragraphs = soup.find_all('p')
        content = self._join_paragraphs(paragraphs)
        return title, content

def get_scraper_for_url(url: str) -> BaseScraper:
    """
    Factory function to return the appropriate scraper based on the domain.
    """
    domain = urlparse(url).netloc.lower()
    
    if 'vnexpress.net' in domain:
        logger.info(f"Domain '{domain}' recognized as VnExpress.")
        return VnExpressScraper()
    elif 'bbc.com' in domain or 'bbc.co.uk' in domain:
        logger.info(f"Domain '{domain}' recognized as BBC.")
        return BBCScraper()
    else:
        logger.info(f"Domain '{domain}' unrecognized. Using GenericScraper.")
        return GenericScraper()

def fetch_article(url: str) -> dict:
    """
    Main entry point for the scraper module. 
    Maintains backward compatibility with main.py.
    """
    scraper = get_scraper_for_url(url)
    return scraper.fetch_article(url)
