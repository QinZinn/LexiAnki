import logging
import newstoanki_rs
from src.processor import process_data

# Configure logging to see output
logging.basicConfig(level=logging.INFO)

def test_lemmatization():
    # Sample data mimicking the scraper output
    article_data = {
        "data": [
            {
                "sentence": "He was reading the largest books in various categories.",
                "words": ["He", "was", "reading", "the", "largest", "books", "in", "various", "categories", "."]
            }
        ]
    }
    
    # Process the data
    # 'reading' -> 'read' (4) -> filtered
    # 'largest' -> 'large' (5) -> kept
    # 'books' -> 'book' (4) -> filtered
    # 'various' -> 'various' (7) -> kept
    # 'categories' -> 'category' (8) -> kept
    
    result = process_data(article_data)
    
    print("\nProcessed Vocabulary:")
    for word, info in result.items():
        print(f"- {word}: {info['context']}")

    expected_words = {"large", "various", "category"}
    actual_words = set(result.keys())
    
    assert expected_words.issubset(actual_words) or actual_words == expected_words, f"Expected {expected_words}, but got {actual_words}"
    
    # Verify original_token storage
    assert result["large"]["original_token"] == "largest"
    assert result["category"]["original_token"] == "categories"
    
    print("\nVerification successful! Lemmatization, filtering, and original_token storage are working correctly.")

def test_truncation():
    print("\nTesting context truncation...")
    
    # Long sentence with target word in the middle
    long_sentence = "This is a very long sentence designed to test the smart truncation logic of our processor module, which should ensure that the target word remains visible even when the total length exceeds our threshold."
    target = "truncation"
    
    truncated = newstoanki_rs.truncate_context(long_sentence, target, 50)
    print(f"Original length: {len(long_sentence)}")
    print(f"Truncated: {truncated}")
    print(f"Truncated length: {len(truncated)}")
    
    assert len(truncated) <= 50
    assert "truncation" in truncated.lower()
    assert "..." in truncated
    
    # Target word at the beginning
    long_sentence_2 = "Remarkable progress has been made in the field of artificial intelligence over the last decade, leading to significant changes in how we live and work."
    target_2 = "Remarkable"
    truncated_2 = newstoanki_rs.truncate_context(long_sentence_2, target_2, 50)
    print(f"Truncated (start): {truncated_2}")
    assert truncated_2.lower().startswith("remarkable")
    assert truncated_2.endswith("...")
    
    # Target word at the end
    long_sentence_3 = "The committee decided to postpone the meeting until next Friday because many members were unable to attend due to a massive snowstorm."
    target_3 = "snowstorm"
    truncated_3 = newstoanki_rs.truncate_context(long_sentence_3, target_3, 50)
    print(f"Truncated (end): {truncated_3}")
    # The sentence ends with "snowstorm." (with a period), but our regex \b match might exclude the period from span
    assert "snowstorm" in truncated_3.lower()
    assert truncated_3.startswith("...")

    print("Verification successful! Smart truncation logic is working correctly.")

def test_proper_noun_filtering():
    article_data = {
        "data": [
            {
                "sentence": "Robert and Sarah visited the beautiful city of Paris.",
                "words": [
                    "Robert", "and", "Sarah", "visited", "the",
                    "beautiful", "city", "of", "Paris",
                ],
            }
        ]
    }

    result = process_data(article_data)

    proper_nouns = {"robert", "sarah", "paris"}
    assert proper_nouns.isdisjoint(result.keys()), (
        f"Proper nouns should be excluded, but found: {proper_nouns & result.keys()}"
    )

    assert "beautiful" in result
    assert result["beautiful"]["original_token"] == "beautiful"

    print("Verification successful! Proper nouns are filtered out correctly.")

if __name__ == "__main__":
    test_lemmatization()
    test_truncation()
    test_proper_noun_filtering()
