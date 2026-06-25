import json
import subprocess
import sys
from pathlib import Path

import nltk
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
        "wordnet",
        "omw-1.4",
    ]:
        try:
            if resource.startswith("averaged"):
                nltk.data.find("taggers/" + resource)
            elif resource in ("wordnet", "omw-1.4"):
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


def python_steps_1_6(sentence: str):
    tokens = nltk.word_tokenize(sentence)
    tagged = nltk.pos_tag(tokens)
    out = []
    sentence_start = {tokens[0].lower()} if tokens else set()
    lemmatizer = WordNetLemmatizer()
    for token, tag in tagged:
        tag = coarse_pos(tag)
        if tag in ("NNP", "NNPS"):
            continue
        word_lower = token.lower()
        if token and token[0].isupper() and word_lower not in sentence_start:
            continue
        if not is_valid_word(word_lower):
            continue
        wn_pos = map_to_wordnet_pos(tag)
        if wn_pos:
            lemma = lemmatizer.lemmatize(word_lower, pos=wn_pos)
        else:
            lemma = lemmatizer.lemmatize(word_lower)
        out.append({"token": word_lower, "pos": tag, "lemma": lemma})
    return out


def rust_steps_1_6(sentences):
    root = Path(__file__).resolve().parents[1]
    payload = json.dumps({"sentences": sentences}).encode("utf-8")
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--release", "--bin", "dump_steps_1_6"],
        cwd=root,
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    )
    return json.loads(proc.stdout.decode("utf-8"))


def main():
    ensure_nltk()

    py = [python_steps_1_6(s) for s in SENTENCES]
    rs = rust_steps_1_6(SENTENCES)

    total = len(SENTENCES)
    token_jaccard_sum = 0.0
    pair_jaccard_sum = 0.0
    lemma_match = 0
    lemma_total = 0

    for idx, (p, r) in enumerate(zip(py, rs), 1):
        p_tokens = {x["token"] for x in p}
        r_tokens = {x["token"] for x in r}
        token_union = p_tokens | r_tokens
        token_inter = p_tokens & r_tokens
        token_j = len(token_inter) / len(token_union) if token_union else 1.0
        token_jaccard_sum += token_j

        p_pairs = {(x["token"], x["pos"]) for x in p}
        r_pairs = {(x["token"], x["pos"]) for x in r}
        pair_union = p_pairs | r_pairs
        pair_inter = p_pairs & r_pairs
        pair_j = len(pair_inter) / len(pair_union) if pair_union else 1.0
        pair_jaccard_sum += pair_j

        for px, rx in zip(p, r):
            if px["token"] == rx["token"]:
                lemma_total += 1
                lemma_match += int(px["lemma"] == rx["lemma"])

        print(f"[{idx}] token_jaccard={token_j:.6f} pair_jaccard={pair_j:.6f}")
        print("  python:", p)
        print("  rust  :", r)

    print("---")
    print(f"AVG_TOKEN_JACCARD={token_jaccard_sum/total:.6f}")
    print(f"AVG_PAIR_JACCARD={pair_jaccard_sum/total:.6f}")
    print(f"LEMMA_ACCURACY={lemma_match/lemma_total:.6f}" if lemma_total else "LEMMA_ACCURACY=1.000000")


if __name__ == "__main__":
    sys.exit(main() or 0)
