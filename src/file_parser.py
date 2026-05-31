import os

import docx
import nltk
from nltk.tokenize import sent_tokenize, word_tokenize

_NLTK_SETUP_DONE = False


def _setup_nltk():
    global _NLTK_SETUP_DONE
    if _NLTK_SETUP_DONE:
        return

    for resource, pkg in [
        ("tokenizers/punkt_tab", "punkt_tab"),
        ("tokenizers/punkt",     "punkt"),
    ]:
        try:
            nltk.data.find(resource)
        except (LookupError, OSError):
            nltk.download(pkg, quiet=True)

    _NLTK_SETUP_DONE = True


def parse_local_file(file_path: str) -> dict:
    ext = os.path.splitext(file_path)[1].lower()

    if ext == ".txt":
        with open(file_path, "r", encoding="utf-8") as f:
            raw_text = f.read()
    elif ext == ".docx":
        document = docx.Document(file_path)
        raw_text = " ".join(p.text for p in document.paragraphs)
    else:
        raise ValueError(f"Unsupported file format: {ext}")

    raw_text = (raw_text or "").strip()
    _setup_nltk()

    sentences = sent_tokenize(raw_text)
    article_data = {
        "title": os.path.splitext(os.path.basename(file_path))[0],
        "data": [
            {"sentence": sentence, "words": word_tokenize(sentence)}
            for sentence in sentences
            if sentence.strip()
        ],
    }
    return article_data
