import logging
import nltk
from nltk.corpus import stopwords

logger = logging.getLogger(__name__)

def setup_nltk():
    """
    Ensures that required NLTK datasets/models are downloaded.
    Specifically checks for the 'stopwords' corpus.
    """
    try:
        nltk.data.find('corpora/stopwords')
    except (LookupError, OSError):
        logger.info("Downloading NLTK stopwords corpus...")
        nltk.download('stopwords', quiet=True)

def load_known_words(filepath: str) -> set:
    """
    Reads a list of known words from a text file.
    Each word should be on a new line.
    
    Args:
        filepath (str): Path to the text file containing known words.
        
    Returns:
        set: A set of lowercase known words.
    """
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            known_words = {line.strip().lower() for line in f if line.strip()}
        logger.info(f"Loaded {len(known_words)} known words from {filepath}.")
        return known_words
    except FileNotFoundError:
        logger.warning(f"{filepath} not found. Starting with an empty blacklist.")
        return set()
    except Exception as e:
        logger.error(f"Error reading {filepath}: {e}. Starting with an empty blacklist.")
        return set()

def process_data(article_data: dict, known_words_file: str = "known_words.txt") -> dict:
    """
    Processes the raw tokenized article data to extract target vocabulary.
    
    Filters applied:
    - Lowercase normalization
    - Strip punctuation and non-alphabetic tokens
    - Remove standard English stop words
    - Keep only words with length >= 5
    - Skip words in the "Known Words Blacklist"
    
    Args:
        article_data (dict): The output from the scraper module, containing sentences and words.
        known_words_file (str): Path to the blacklist file. Defaults to "known_words.txt".
    
    Returns:
        dict: A dictionary mapping unique target words to their context (the original sentence).
              Example: {"prodigy": {"context": "The original string..."}}
    """
    setup_nltk()
    
    stop_words = set(stopwords.words('english'))
    known_words = load_known_words(known_words_file)
    
    unique_vocabulary = {}
    
    data_list = article_data.get('data', [])
    
    for item in data_list:
        original_sentence = item.get('sentence', '')
        words = item.get('words', [])
        
        for token in words:
            # 1. Normalization
            word_lower = token.lower()
            
            # 2. Cleaning: Must be alphabetic
            if not word_lower.isalpha():
                continue
                
            # 3. Length constraint
            if len(word_lower) < 5:
                continue
                
            # 4. Stop-words removal
            if word_lower in stop_words:
                continue
            
            # 5. Known words blacklist removal
            if word_lower in known_words:
                continue
                
            # 6. Deduplication & Context mapping
            # If the word is already in our dictionary, we skip to keep the *first* encountered context
            if word_lower not in unique_vocabulary:
                unique_vocabulary[word_lower] = {
                    "context": original_sentence
                }
                
    logger.info(f"NLP Processing complete. Found {len(unique_vocabulary)} unique target words after filtering.")
    
    return unique_vocabulary
