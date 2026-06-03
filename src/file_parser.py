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
    elif ext == ".pptx":
        from pptx import Presentation

        presentation = Presentation(file_path)
        texts = []
        for slide in presentation.slides:
            for shape in slide.shapes:
                if not getattr(shape, "has_text_frame", False):
                    continue
                text_frame = shape.text_frame
                if not text_frame:
                    continue
                for paragraph in text_frame.paragraphs:
                    if paragraph.text:
                        texts.append(paragraph.text)
        raw_text = " ".join(texts)
    elif ext == ".pdf":
        import fitz

        doc = fitz.open(file_path)
        try:
            raw_text = " ".join(page.get_text() for page in doc)
        finally:
            doc.close()
    else:
        raise ValueError(
            f"Unsupported file format: {ext}. Supported: .txt, .docx, .pptx, .pdf"
        )

    raw_text = " ".join((raw_text or "").split())
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
