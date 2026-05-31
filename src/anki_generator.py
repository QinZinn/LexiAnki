import logging
import hashlib
import genanki
import regex as re

logger = logging.getLogger(__name__)

def generate_anki_deck(enriched_data: dict, output_filename: str = "English_News_Vocab.apkg") -> str:
    """
    Generates an Anki deck (.apkg) from the enriched vocabulary data.
    
    Args:
        enriched_data (dict): Dictionary containing vocabulary.
                              Example: {"word": {"context": "...", "part_of_speech": "...", "definition": "..."}}
        output_filename (str): The name of the output .apkg file.
                              
    Returns:
        str: Path to the generated Anki deck file.
    """
    logger.info("Generating Anki deck...")

    # Define a clean, modern CSS string
    css = """
    .card {
        font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
        font-size: 20px;
        text-align: center;
        color: #333333;
        background-color: #f9f9fb;
        padding: 20px;
        line-height: 1.6;
    }
    .word {
        font-size: 36px;
        font-weight: bold;
        color: #2c3e50;
        margin-bottom: 15px;
    }
    hr {
        border: 0;
        height: 1px;
        background: #e0e0e0;
        margin: 20px 0;
    }
    .pos {
        font-style: italic;
        color: #7f8c8d;
        font-size: 18px;
        margin-bottom: 10px;
    }
    .definition {
        font-size: 20px;
        color: #34495e;
        margin-bottom: 20px;
    }
    .context {
        font-size: 18px;
        color: #555555;
        background-color: #ffffff;
        padding: 15px;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.05);
        border-left: 4px solid #3498db;
        text-align: left;
    }
    .highlight {
        font-weight: bold;
        color: #e74c3c;
    }
    """

    # Create Anki Model
    model_id = int(hashlib.md5(b"EnglishNewsVocabModel_v1").hexdigest()[:8], 16)
    my_model = genanki.Model(
        model_id,
        'English News Vocab Model',
        fields=[
            {'name': 'Word'},
            {'name': 'PartOfSpeech'},
            {'name': 'Definition'},
            {'name': 'Context'},
        ],
        templates=[
            {
                'name': 'Card 1',
                'qfmt': '<div class="word">{{Word}}</div>',
                'afmt': '{{FrontSide}}<hr id="answer"><div class="pos">{{PartOfSpeech}}</div><div class="definition">{{Definition}}</div><div class="context">{{Context}}</div>',
            },
        ],
        css=css
    )

    # Create Anki Deck
    deck_id = int(hashlib.md5(output_filename.encode()).hexdigest()[:8], 16)
    my_deck = genanki.Deck(
        deck_id,
        'English News Vocabulary'
    )

    # Add notes to the deck
    for word, details in enriched_data.items():
        original_context = details.get("context", "")
        original_token = details.get("original_token", word)
        
        # Highlight the target word in the context sentence (case-insensitive replace)
        # Using regex to wrap the original token with a highlight span while keeping original casing
        highlighted_context = re.sub(
            r'\b(' + re.escape(original_token) + r')\b', 
            r'<span class="highlight">\1</span>', 
            original_context, 
            flags=re.IGNORECASE
        )

        note = genanki.Note(
            model=my_model,
            guid=genanki.guid_for(output_filename, word),
            fields=[
                word, 
                details.get("part_of_speech", ""), 
                details.get("definition", ""), 
                highlighted_context
            ]
        )
        my_deck.add_note(note)

    # Export
    genanki.Package(my_deck).write_to_file(output_filename)
    
    logger.info(f"Deck saved successfully at {output_filename}")
    return output_filename
