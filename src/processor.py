import logging
import nltk
import newstoanki_rs
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

def process_data(article_data: dict, known_words_file: str = "known_words.txt") -> dict:
    """
    Processes the raw tokenized article data to extract target vocabulary.
    
    Filters applied:
    - Skip proper nouns (NNP, NNPS) before lemmatization
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
    
    unique_vocabulary = {}
    
    data_list = article_data.get('data', [])
    
    for item in data_list:
        original_sentence = item.get('sentence', '')
        tokens = item.get('words', [])
        
        # 1. POS Tagging
        tagged_tokens = nltk.pos_tag(tokens)
        
        for token, tag in tagged_tokens:
            # 2. Skip proper nouns (people, places, organizations, etc.)
            if tag in ('NNP', 'NNPS'):
                continue

            # 3. Normalization
            word_lower = token.lower()
            
            # 4. Fail-fast validation (pre-lemmatization): skip short/invalid tokens early to save CPU.
            # 5. Post-lemmatization validation: ensure the resulting base form still meets our constraints.
            if not newstoanki_rs.is_valid_word(word_lower):
                continue
            
            # 6. Map POS tag for lemmatization
            wn_pos = get_wordnet_pos(tag)
            
            # 7. Lemmatization
            if wn_pos:
                word_lemma = lemmatizer.lemmatize(word_lower, pos=wn_pos)
            else:
                # Default to WordNet's noun behavior when POS is unknown.
                word_lemma = lemmatizer.lemmatize(word_lower)  # defaults to noun

            # 8. Validation (post-lemmatization)
            if not newstoanki_rs.is_valid_word(word_lemma):
                continue
                
            # 9. Stop-words removal
            if word_lemma in stop_words:
                continue
            
            # 10. Known words blacklist removal
            if word_lemma in known_words:
                continue
                
            # 11. Deduplication & Context mapping
            if word_lemma not in unique_vocabulary:
                truncated_context = newstoanki_rs.truncate_context(original_sentence, token, 150)
                unique_vocabulary[word_lemma] = {
                    "context": truncated_context,
                    "pos": wn_pos,  # Store for dictionary lookup
                    "original_token": token
                }
                
    logger.info(f"NLP Processing complete. Found {len(unique_vocabulary)} unique target words after filtering.")
    
    return unique_vocabulary
