import argparse
import logging
import sys
import os
from itertools import islice

from src.scraper import fetch_article
from src.file_parser import parse_local_file
from src.processor import process_data, update_known_words
from src.dictionary_lookup import lookup_definitions
from src.anki_generator import generate_anki_deck
from src.exporter import export_to_csv

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
        help="The URL of the English news article to process."
    )
    parser.add_argument(
        "--file",
        type=str,
        help="Path to a local .txt or .docx file to extract vocabulary from."
    )
    parser.add_argument(
        "--output", 
        type=str, 
        default="English_News_Vocab.apkg",
        help="The name of the output .apkg file (default: English_News_Vocab.apkg)."
    )
    parser.add_argument(
        "--mark-known",
        action="store_true",
        help="If provided, all successfully extracted words will be added to known_words.txt."
    )
    parser.add_argument(
        "--add-known",
        type=str,
        help="A comma-separated list of words to add to known_words.txt. If provided, the tool exits after adding."
    )
    parser.add_argument(
        "--max-words",
        type=int,
        default=0,
        help="Maximum number of words to extract. 0 means no limit."
    )
    parser.add_argument(
        "--export-csv",
        type=str,
        help="Export the extracted vocabulary to a CSV file."
    )
    args = parser.parse_args()

    # Handle --add-known logic
    if args.add_known:
        new_words = [w.strip() for w in args.add_known.split(",") if w.strip()]
        if new_words:
            update_known_words(new_words)
            logger.info(f"Added {len(new_words)} words to known_words.txt.")
        else:
            logger.warning("No valid words provided to --add-known.")
        if args.url or args.file:
            logger.warning("--add-known exits immediately; --url/--file will NOT be processed in this run.")
        sys.exit(0)

    if bool(args.url) == bool(args.file):
        logger.error("Exactly one of --url or --file must be provided.")
        sys.exit(2)

    output_filename = args.output
    
    logger.info("==================================================")
    logger.info("Starting Vocabulary Extraction Pipeline")
    if args.url:
        logger.info(f"Target URL: {args.url}")
    else:
        logger.info(f"Target File: {args.file}")
    logger.info(f"Output File: {output_filename}")
    if args.max_words > 0:
        logger.info(f"Limit: {args.max_words} words")
    logger.info("==================================================")

    try:
        if args.file:
            logger.info("--- Phase 1: Parsing Local File ---")
            article_data = parse_local_file(args.file)
        else:
            logger.info("--- Phase 1: Scraping Article ---")
            article_data = fetch_article(args.url)

        if not article_data or not article_data.get('data'):
            logger.error("Failed to extract content or input is empty.")
            sys.exit(1)

        logger.info(f"Title: '{article_data.get('title', 'Unknown Title')}'")
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

        # Apply --max-words limit if provided
        if args.max_words > 0 and len(enriched_data) > args.max_words:
            logger.info(f"Applying limit of {args.max_words} words.")
            # slice the dictionary
            enriched_data = dict(islice(enriched_data.items(), args.max_words))

        # Handle --export-csv logic
        if args.export_csv:
            logger.info(f"--- Exporting to CSV: {args.export_csv} ---")
            export_to_csv(enriched_data, args.export_csv)

        # Phase 4: Anki Generation
        logger.info("--- Phase 4: Generating Anki Deck ---")
        deck_path = generate_anki_deck(enriched_data, output_filename)
        
        if os.path.exists(deck_path):
            logger.info("==================================================")
            logger.info(f"Pipeline completed successfully!")
            logger.info(f"Anki deck saved to: {deck_path}")
            logger.info("==================================================")
            
            # Handle --mark-known logic
            if args.mark_known:
                logger.info("--- Phase 5: Updating Known Words ---")
                extracted_words = list(enriched_data.keys())
                if extracted_words:
                    update_known_words(extracted_words)
                else:
                    logger.warning("No words to mark as known.")
        else:
            logger.error("Failed to generate Anki package file.")
            sys.exit(1)

    except Exception as e:
        logger.error(f"An unexpected error occurred: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
