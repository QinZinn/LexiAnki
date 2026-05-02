# News-to-Anki CLI 🚀

A professional Python-based CLI tool designed to streamline language learning by automating the extraction of target vocabulary from news articles and generating ready-to-use Anki flashcards (.apkg).

![Anki Demo](assets/preview.png)

## 🌟 Key Features

- **Smart Scraping**: Built-in support for **VnExpress (English)** and **BBC News**, featuring a robust retry mechanism with exponential backoff and randomized User-Agents to prevent blocking.
- **Context-Aware NLP**: 
  - **POS Tagging**: Utilizes `nltk.pos_tag` to understand the grammatical context of each word.
  - **Smart Lemmatization**: Performs accurate base-form reduction based on Part-of-Speech.
  - **Context-Aware Definitions**: Fetches the most relevant WordNet definitions matching the word's usage in the sentence.
- **Optimized Anki UX**: Implements **Smart Context Truncation** to keep flashcards readable while ensuring the target word remains perfectly centered in its context.
- **Accented Character Support**: Correctly processes loanwords and names with accents (e.g., *café*, *résumé*).
- **Modern Workflow**: Managed by `uv` for lightning-fast dependency management and consistent environments. Supports Python 3.10+.
- **Automated Blacklist**: Effortlessly manage your `known_words.txt` via CLI flags to skip words you've already mastered.
- **CI/CD Ready**: Fully integrated with GitHub Actions for automated testing and quality assurance.

## 🛠️ Installation

This project uses [uv](https://github.com/astral-sh/uv) for dependency management.

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/NewsToAnki.git
   cd NewsToAnki/Backend
   ```

2. **Sync dependencies**:
   ```bash
   uv sync
   ```

## 🚀 Usage

Run the tool from the `Backend/` directory using `uv run`.

### Basic Command
```bash
uv run main.py --url "https://e.vnexpress.net/news/news/education/vietnam-wins-four-gold-medals-at-international-chemistry-olympiad-4775486.html"
```

### Custom Output
```bash
uv run main.py --url "https://www.bbc.com/news/articles/c0jje79z7jno" --output "bbc_vocab.apkg"
```

### Managing Known Words
Automate the update of `known_words.txt` to streamline your learning process:

- **Auto-mark as known**: Append all extracted words from an article to your blacklist after generation.
  ```bash
  uv run main.py --url "https://..." --mark-known
  ```
- **Manual add**: Add a list of words directly to the blacklist and exit.
  ```bash
  uv run main.py --add-known "essential, remarkable, challenge"
  ```

### Arguments
- `--url`: The URL of the news article to process (required unless using `--add-known`).
- `--output`: (Optional) The name of the output `.apkg` file.
- `--mark-known`: (Optional) Automatically add extracted words to `known_words.txt`.
- `--add-known`: (Optional) A comma-separated list of words to add to `known_words.txt`.

## 🧪 Testing

The project includes a comprehensive test suite for NLP processing and truncation logic.

```bash
uv run test_processor.py
```

## 📂 Project Structure

```text
NewsToAnki/
├── .github/workflows/       # CI/CD (GitHub Actions)
├── Backend/
│   ├── main.py              # CLI Entry Point
│   ├── test_processor.py    # Unit Tests
│   ├── known_words.txt      # Vocabulary Blacklist
│   ├── pyproject.toml       # uv Configuration
│   ├── uv.lock              # Lockfile
│   └── src/                 # Core Package
│       ├── __init__.py
│       ├── scraper.py       # Robust Scraper (Retries, Fake UA)
│       ├── processor.py     # POS Tagging, Smart Truncation
│       ├── dictionary_lookup.py # Context-aware WordNet lookup
│       └── anki_generator.py    # .apkg Generation
├── assets/                  # Documentation Assets
└── README.md                # Project Documentation
```

## 📄 License
MIT License. See [LICENSE](LICENSE) for details.
