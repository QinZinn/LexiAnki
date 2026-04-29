import argparse
import logging
import sys
import os

from src.scraper import fetch_article
from src.processor import process_data
from src.dictionary_lookup import lookup_definitions
from src.anki_generator import generate_anki_deck

def setup_logging():
    """Configures the logging format and level."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.StreamHandler(sys.stdout)
        ]
    )

def main():
    setup_logging()
    logger = logging.getLogger(__name__)

    # CLI setup
    parser = argparse.ArgumentParser(
        description="Automated pipeline to extract vocabulary from English news articles to Anki flashcards."
    )
    parser.add_argument(
        "--url", 
        type=str, 
        required=True, 
        help="The URL of the English news article to process."
    )
    parser.add_argument(
        "--output", 
        type=str, 
        default="English_News_Vocab.apkg",
        help="The name of the output .apkg file (default: English_News_Vocab.apkg)."
    )
    args = parser.parse_args()

    url = args.url
    output_filename = args.output
    
    logger.info("==================================================")
    logger.info("Starting Vocabulary Extraction Pipeline")
    logger.info(f"Target URL: {url}")
    logger.info(f"Output File: {output_filename}")
    logger.info("==================================================")

    try:
        # Phase 1: Scraper
        logger.info("--- Phase 1: Scraping Article ---")
        article_data = fetch_article(url)
        if not article_data or not article_data.get('data'):
            logger.error("Failed to scrape article or article is empty.")
            sys.exit(1)

        logger.info(f"Article successfully scraped: '{article_data.get('title', 'Unknown Title')}'")
        logger.info(f"Total sentences extracted: {len(article_data['data'])}")

        # Phase 2: Processor
        logger.info("--- Phase 2: NLP Processing / Filtering ---")
        processed_data = process_data(article_data)
        if not processed_data:
            logger.warning("No target vocabulary found. Exiting.")
            sys.exit(0)

        logger.info(f"Extracted {len(processed_data)} unique candidate words.")

        # Phase 3: Dictionary Lookup
        logger.info("--- Phase 3: Dictionary Lookup (Offline via WordNet) ---")
        enriched_data = lookup_definitions(processed_data)
        if not enriched_data:
            logger.warning("No definitions found for the extracted vocabulary. Exiting.")
            sys.exit(0)

        logger.info(f"Enriched {len(enriched_data)} words with definitions.")

        # Phase 4: Anki Generation
        logger.info("--- Phase 4: Generating Anki Deck ---")
        deck_path = generate_anki_deck(enriched_data, output_filename)
        
        if os.path.exists(deck_path):
            logger.info("==================================================")
            logger.info(f"Pipeline completed successfully!")
            logger.info(f"Anki deck saved to: {deck_path}")
            logger.info("==================================================")
        else:
            logger.error("Failed to generate Anki package file.")
            sys.exit(1)

    except Exception as e:
        logger.error(f"An unexpected error occurred: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
