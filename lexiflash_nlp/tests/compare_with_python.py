import json
import subprocess
import sys
from pathlib import Path

import nltk
from nltk.corpus import stopwords
from nltk.corpus import wordnet
from nltk.stem import WordNetLemmatizer


SENTENCES = [
    "She was reading the largest books in various categories.",
    "The remarkable committee postponed the meeting until Friday.",
    "Beautiful cities inspire curious travelers and careful planners.",
    "Researchers are analyzing multilingual datasets for robust tagging.",
    "A clever parser should ignore malformed tokens gracefully.",
    "Robert and Sarah visited the beautiful city of Paris.",
    "American researchers visited Boston during Vietnamese cultural events.",
    "Our American guide described Parisian architecture to curious visitors.",
]


def ensure_nltk():
    for resource in [
        "averaged_perceptron_tagger_eng",
        "averaged_perceptron_tagger",
        "punkt",
        "punkt_tab",
        "stopwords",
        "wordnet",
        "omw-1.4",
    ]:
        try:
            if resource.startswith("averaged"):
                nltk.data.find("taggers/" + resource)
            elif resource in ("wordnet", "omw-1.4", "stopwords"):
                nltk.data.find("corpora/" + resource)
            else:
                nltk.data.find("tokenizers/" + resource)
        except Exception:
            nltk.download(resource, quiet=True)


def coarse_pos(tag: str) -> str:
    return tag.split(":", 1)[0]

def map_to_wordnet_pos(treebank_tag: str):
    if treebank_tag.startswith("J"):
        return wordnet.ADJ
    if treebank_tag.startswith("V"):
        return wordnet.VERB
    if treebank_tag.startswith("N"):
        return wordnet.NOUN
    if treebank_tag.startswith("R"):
        return wordnet.ADV
    return None


def is_valid_word(word: str) -> bool:
    if len(word) < 5:
        return False
    return word.isalpha()


PROPER_LEXNAMES = frozenset({
    "noun.person",
    "noun.location",
    "noun.group",
    "noun.object",
})

def load_known_words() -> set:
    repo_root = Path(__file__).resolve().parents[2]
    path = repo_root / "known_words.txt"
    if not path.exists():
        return set()
    return {line.strip().lower() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()}

def python_full(sentences):
    lemmatizer = WordNetLemmatizer()
    stop_words = set(stopwords.words("english"))
    known_words = load_known_words()
    unique = {}
    lemmatizer = WordNetLemmatizer()
    for sentence in sentences:
        tokens = nltk.word_tokenize(sentence)
        tagged = nltk.pos_tag(tokens)
        sentence_start = {tokens[0].lower()} if tokens else set()

        for token, tag in tagged:
            tag = coarse_pos(tag)
            if tag in ("NNP", "NNPS"):
                continue

            word_lower = token.lower()
            if not is_valid_word(word_lower):
                continue

            wn_pos = map_to_wordnet_pos(tag)
            if wn_pos:
                lemma = lemmatizer.lemmatize(word_lower, pos=wn_pos)
            else:
                lemma = lemmatizer.lemmatize(word_lower)

            if token and token[0].isupper() and word_lower not in sentence_start:
                continue

            if token and token[0].isupper():
                synsets = wordnet.synsets(lemma)
                if synsets and synsets[0].lexname() in PROPER_LEXNAMES:
                    continue

            if not is_valid_word(lemma):
                continue

            if lemma in stop_words:
                continue

            if lemma in known_words:
                continue

            if lemma not in unique:
                entry = {"original_token": token}
                if wn_pos is not None:
                    entry["pos"] = wn_pos
                unique[lemma] = entry

    return unique

def rust_full(sentences):
    root = Path(__file__).resolve().parents[1]
    payload = json.dumps({"sentences": sentences}).encode("utf-8")
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--release", "--bin", "dump_steps_1_12"],
        cwd=root,
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    )
    return json.loads(proc.stdout.decode("utf-8"))




def main():
    ensure_nltk()

    py_vocab = python_full(SENTENCES)
    rs_vocab = rust_full(SENTENCES)

    py_lemma_set = set(py_vocab.keys())
    rs_lemma_set = {item["lemma"] for item in rs_vocab}
    intersection = py_lemma_set & rs_lemma_set
    union = py_lemma_set | rs_lemma_set
    recall = len(intersection) / len(py_lemma_set) if py_lemma_set else 1.0
    jaccard = len(intersection) / len(union) if union else 1.0

    print("---")
    print(f"PYTHON_LEMMA_COUNT={len(py_lemma_set)}")
    print(f"RUST_LEMMA_COUNT={len(rs_lemma_set)}")
    print(f"INTERSECTION_COUNT={len(intersection)}")
    print(f"UNION_COUNT={len(union)}")
    print(f"LEMMA_SET_RECALL={recall:.6f}")
    print(f"LEMMA_SET_JACCARD={jaccard:.6f}")
    print(f"PYTHON_ONLY={sorted(py_lemma_set - rs_lemma_set)}")
    print(f"RUST_ONLY={sorted(rs_lemma_set - py_lemma_set)}")


if __name__ == "__main__":
    sys.exit(main() or 0)
