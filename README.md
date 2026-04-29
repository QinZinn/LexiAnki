# News-to-Anki CLI

A professional Python CLI tool that automates the extraction of target vocabulary from English news articles and generates Anki flashcards for efficient language learning.

## Features
- **Article Scraping**: Extracts text and title from any English news URL.
- **NLP Processing**: Tokenizes sentences and words, filters out common stopwords, and identifies candidate vocabulary.
- **Offline Dictionary Lookup**: Enriches vocabulary with definitions and parts of speech using WordNet (NLTK).
- **Anki Deck Generation**: Packages everything into a ready-to-import `.apkg` file.
- **Customizable Blacklist**: Uses a `known_words.txt` file to skip words you already know.

## Prerequisites
- Python 3.8+
- [Anki](https://apps.ankiweb.net/) (to import the generated decks)

## Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/NewsToAnki.git
   cd NewsToAnki/Backend
   ```

2. **Create a virtual environment (optional but recommended)**:
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   ```

3. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

## Usage

Run the tool from the `Backend` directory using `python main.py`.

### Basic Command
```bash
python main.py --url "https://www.bbc.com/news/world-61343815"
```

### Custom Output Filename
```bash
python main.py --url "https://www.bbc.com/news/world-61343815" --output "my_vocab.apkg"
```

### Arguments
- `--url`: (Required) The URL of the English news article to process.
- `--output`: (Optional) The name of the output `.apkg` file (default: `English_News_Vocab.apkg`).

## Project Structure
```
Backend/
├── main.py              # Entry point for the CLI
├── known_words.txt      # Blacklist for skipping known words
├── requirements.txt     # Python dependencies
└── src/                 # Core logic modules
    ├── __init__.py
    ├── anki_generator.py
    ├── dictionary_lookup.py
    ├── processor.py
    └── scraper.py
```

## How it Works
1. **Scraper**: Uses `requests` and `BeautifulSoup` to fetch and parse the article.
2. **Processor**: Uses `nltk` for tokenization and filtering. It skips words shorter than 5 characters, standard stop words, and words listed in `known_words.txt`.
3. **Dictionary**: Uses `WordNet` via `nltk` to find definitions and parts of speech.
4. **Generator**: Uses `genanki` to create Anki notes and package them into a deck.
