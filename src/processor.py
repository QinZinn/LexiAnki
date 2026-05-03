import logging
import nltk
import re
from nltk.corpus import stopwords
from nltk.stem import WordNetLemmatizer
from nltk.corpus import wordnet

logger = logging.getLogger(__name__)

def setup_nltk():
    """
    Ensures that required NLTK datasets/models are downloaded.
    Specifically checks for the 'stopwords', 'wordnet', 'omw-1.4', 'punkt', and 'averaged_perceptron_tagger' corpora.
    """
    resources = [
        ('corpora/stopwords', 'stopwords'),
        ('corpora/wordnet', 'wordnet'),
        ('corpora/omw-1.4', 'omw-1.4'),
        ('taggers/averaged_perceptron_tagger', 'averaged_perceptron_tagger'),
        ('taggers/averaged_perceptron_tagger_eng', 'averaged_perceptron_tagger_eng')
    ]
    
    for path, pkg in resources:
        try:
            nltk.data.find(path)
        except (LookupError, OSError):
            logger.info(f"Downloading NLTK {pkg} corpus...")
            nltk.download(pkg, quiet=True)

def get_wordnet_pos(treebank_tag):
    """
    Maps NLTK Treebank POS tags to WordNet POS tags.
    """
    if treebank_tag.startswith('J'):
        return wordnet.ADJ
    elif treebank_tag.startswith('V'):
        return wordnet.VERB
    elif treebank_tag.startswith('N'):
        return wordnet.NOUN
    elif treebank_tag.startswith('R'):
        return wordnet.ADV
    else:
        return None

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

def update_known_words(new_words_list: list, filepath: str = "known_words.txt"):
    """
    Appends new words to the known words file, removes duplicates, and sorts alphabetically.
    
    Args:
        new_words_list (list): List of new words to add.
        filepath (str): Path to the text file containing known words.
    """
    existing_words = load_known_words(filepath)
    # Combine, lowercase, and remove duplicates
    all_words = existing_words.union({w.strip().lower() for w in new_words_list if w.strip()})
    
    # Sort alphabetically
    sorted_words = sorted(list(all_words))
    
    try:
        with open(filepath, 'w', encoding='utf-8') as f:
            for word in sorted_words:
                f.write(f"{word}\n")
        logger.info(f"Successfully updated {filepath}. Total known words: {len(sorted_words)}.")
    except Exception as e:
        logger.error(f"Error writing to {filepath}: {e}")

def truncate_context(sentence: str, target_word: str, max_length: int = 150) -> str:
    """
    Truncates a sentence to a maximum length while keeping the target word visible.
    
    Args:
        sentence (str): The full context sentence.
        target_word (str): The word that must remain visible.
        max_length (int): The maximum length of the resulting string.
        
    Returns:
        str: The truncated sentence with '...' if cut off.
    """
    if len(sentence) <= max_length:
        return sentence

    # Find the position of the target word (case-insensitive)
    # Use regex to find word boundaries to avoid partial matches (e.g., 'read' in 'reading')
    pattern = re.compile(rf'\b{re.escape(target_word)}\b', re.IGNORECASE)
    match = pattern.search(sentence)
    
    if not match:
        # Fallback if regex fails for some reason: just return the first max_length chars
        return sentence[:max_length-3] + "..."

    start_idx, end_idx = match.span()
    
    # Calculate how much space we have around the word
    remaining_length = max_length - (end_idx - start_idx) - 6  # 6 for two '...'
    
    # We want to show roughly equal amount of text before and after the word
    prefix_length = remaining_length // 2
    suffix_length = remaining_length - prefix_length
    
    # Determine the actual start and end indices
    new_start = max(0, start_idx - prefix_length)
    new_end = min(len(sentence), end_idx + suffix_length)
    
    # Adjust if we hit the boundaries
    if new_start == 0:
        new_end = min(len(sentence), max_length - 3)
    elif new_end == len(sentence):
        new_start = max(0, len(sentence) - (max_length - 3))
        
    result = sentence[new_start:new_end]
    
    # Add ellipses if needed
    if new_start > 0:
        result = "..." + result
    if new_end < len(sentence):
        result = result + "..."
        
    return result

def process_data(article_data: dict, known_words_file: str = "known_words.txt") -> dict:
    """
    Processes the raw tokenized article data to extract target vocabulary.
    
    Filters applied:
    - Lowercase normalization
    - Strip punctuation and allow accented characters (e.g., café, résumé)
    - POS Tagging for context-aware lemmatization and definition lookup
    - Lemmatization (base form reduction)
    - Remove standard English stop words
    - Keep only words with length >= 5
    - Skip words in the "Known Words Blacklist"
    - Truncate context for Anki UX
    
    Args:
        article_data (dict): The output from the scraper module, containing sentences and words.
        known_words_file (str): Path to the blacklist file. Defaults to "known_words.txt".
    
    Returns:
        dict: A dictionary mapping unique target words to their context and POS.
              Example: {"prodigy": {"context": "...", "pos": "n"}}
    """
    setup_nltk()
    
    lemmatizer = WordNetLemmatizer()
    stop_words = set(stopwords.words('english'))
    known_words = load_known_words(known_words_file)
    
    # Regex to allow accented characters
    accented_regex = re.compile(r'^[a-zA-ZÀ-ỹ]+$')
    
    unique_vocabulary = {}
    
    data_list = article_data.get('data', [])
    
    for item in data_list:
        original_sentence = item.get('sentence', '')
        tokens = item.get('words', [])
        
        # 1. POS Tagging
        tagged_tokens = nltk.pos_tag(tokens)
        
        for token, tag in tagged_tokens:
            # 2. Normalization
            word_lower = token.lower()
            
            # 3. Cleaning: Allow alphabetic and accented characters
            if not accented_regex.match(word_lower):
                continue
            
            # 4. Map POS tag for lemmatization
            wn_pos = get_wordnet_pos(tag)
            
            # 5. Lemmatization
            # If we have a specific POS, use it. Otherwise, perform triple-pass as fallback
            if wn_pos:
                word_lemma = lemmatizer.lemmatize(word_lower, pos=wn_pos)
            else:
                word_lemma = lemmatizer.lemmatize(lemmatizer.lemmatize(lemmatizer.lemmatize(word_lower, pos='v'), pos='n'), pos='a')
                
            # 6. Length constraint
            if len(word_lemma) < 5:
                continue
                
            # 7. Stop-words removal
            if word_lemma in stop_words:
                continue
            
            # 8. Known words blacklist removal
            if word_lemma in known_words:
                continue
                
            # 9. Deduplication & Context mapping
            if word_lemma not in unique_vocabulary:
                # Truncate context for Anki UX
                truncated_context = truncate_context(original_sentence, token)
                unique_vocabulary[word_lemma] = {
                    "context": truncated_context,
                    "pos": wn_pos,  # Store for dictionary lookup
                    "original_token": token
                }
                
    logger.info(f"NLP Processing complete. Found {len(unique_vocabulary)} unique target words after filtering.")
    
    return unique_vocabulary
