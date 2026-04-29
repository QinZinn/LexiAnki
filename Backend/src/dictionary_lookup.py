import logging
import nltk
from nltk.corpus import wordnet

logger = logging.getLogger(__name__)

def setup_wordnet():
    """
    Ensures that required NLTK datasets/models are downloaded.
    Specifically checks for 'wordnet' and 'omw-1.4'.
    """
    try:
        nltk.data.find('corpora/wordnet')
        nltk.data.find('corpora/omw-1.4')
    except (LookupError, OSError):
        logger.info("Downloading NLTK wordnet and omw-1.4 corpora...")
        nltk.download('wordnet', quiet=True)
        nltk.download('omw-1.4', quiet=True)

def lookup_definitions(processed_data: dict) -> dict:
    """
    Looks up definitions and parts of speech for words using nltk.corpus.wordnet.
    
    Args:
        processed_data (dict): Dictionary mapping words to their context.
                               Example: {"genius": {"context": "..."}}
                               
    Returns:
        dict: Enriched dictionary with part_of_speech and definition.
              Example: {
                  "genius": {
                      "context": "...",
                      "part_of_speech": "Noun",
                      "definition": "..."
                  }
              }
    """
    setup_wordnet()
    enriched_data = {}
    
    # Map WordNet POS characters to readable formats
    pos_map = {
        'n': 'Noun',
        'v': 'Verb',
        'a': 'Adjective',
        's': 'Adjective',  # satellite adjective
        'r': 'Adverb'
    }
    
    for word, data in processed_data.items():
        logger.info(f"Looking up definition for: {word}...")
        try:
            synsets = wordnet.synsets(word)
            if synsets:
                # Get the first synset (the most common meaning)
                syn = synsets[0]
                
                # Extract Part of Speech and map it to a full word
                pos_char = syn.pos()
                full_pos = pos_map.get(pos_char, 'Unknown')
                
                # Extract definition
                definition = syn.definition()
                
                enriched_data[word] = {
                    "context": data["context"],
                    "part_of_speech": full_pos,
                    "definition": definition
                }
            else:
                logger.warning(f"No definition found for '{word}' in WordNet. Skipping...")
                
        except Exception as e:
            logger.error(f"Error looking up '{word}': {e}. Skipping...")
            
    logger.info(f"Dictionary lookup complete. Successfully enriched {len(enriched_data)} words.")
    
    return enriched_data
