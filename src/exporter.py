import csv
import logging

logger = logging.getLogger(__name__)

def export_to_csv(vocab_data, filename):
    """
    Exports the enriched vocabulary data to a CSV file.
    
    Args:
        vocab_data (dict): Enriched dictionary with part_of_speech and definition.
                           Example: {
                               "word": {
                                   "context": "...",
                                   "part_of_speech": "Noun",
                                   "definition": "..."
                               }
                           }
        filename (str): The name of the CSV file to write.
    """
    if not vocab_data:
        logger.warning("No data to export to CSV.")
        return

    headers = ["Word", "POS", "Definition", "Context"]
    
    try:
        with open(filename, mode='w', encoding='utf-8', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=headers)
            writer.writeheader()
            
            for word, data in vocab_data.items():
                writer.writerow({
                    "Word": word,
                    "POS": data.get("part_of_speech", "Unknown"),
                    "Definition": data.get("definition", ""),
                    "Context": data.get("context", "")
                })
        
        logger.info(f"Successfully exported {len(vocab_data)} words to {filename}.")
    except Exception as e:
        logger.error(f"Error exporting to CSV {filename}: {e}")
