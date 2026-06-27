use anyhow::{Context, Result, bail};
use lexianki_nlp::{VocabularyEntry, WordnetPos};
use rusqlite::{Connection, params};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "lexiflash";
const DB_FILE_NAME: &str = "lexiflash.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckSummary {
    pub id: i64,
    pub title: String,
    pub source_type: String,
    pub source_path: String,
    pub created_at: String,
    pub sentence_count: i64,
    pub vocabulary_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudySnapshot {
    pub learned_total: i64,
    pub streak_days: i64,
    pub due_today: i64,
}

pub fn default_db_path() -> Result<PathBuf> {
    let base_dir = dirs::data_local_dir().context("could not resolve OS data directory")?;
    Ok(base_dir.join(APP_DIR_NAME).join(DB_FILE_NAME))
}

pub fn init_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create database directory '{}'", parent.display()))?;
    }

    let conn = Connection::open(path)
        .with_context(|| format!("failed to open SQLite database '{}'", path.display()))?;
    init_connection(&conn)?;
    Ok(conn)
}

pub fn save_deck(
    conn: &Connection,
    title: &str,
    source_type: &str,
    source_path: &str,
    sentence_count: usize,
    entries: &[VocabularyEntry],
) -> Result<i64> {
    validate_source_type(source_type)?;

    conn.execute_batch("BEGIN IMMEDIATE TRANSACTION;")
        .context("failed to begin deck save transaction")?;

    let save_result = (|| -> Result<i64> {
        conn.execute(
            "INSERT INTO decks (title, source_type, source_path, sentence_count)
             VALUES (?1, ?2, ?3, ?4)",
            params![title, source_type, source_path, sentence_count as i64],
        )
        .context("failed to insert deck")?;

        let deck_id = conn.last_insert_rowid();

        for entry in entries {
            conn.execute(
                "INSERT INTO vocabulary_entries
                 (deck_id, lemma, context, original_token, wordnet_pos)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    deck_id,
                    entry.lemma,
                    entry.context,
                    entry.original_token,
                    wordnet_pos_to_sql(entry.wordnet_pos),
                ],
            )
            .with_context(|| {
                format!(
                    "failed to insert vocabulary entry '{}' for deck {}",
                    entry.lemma, deck_id
                )
            })?;

            let vocabulary_entry_id = conn.last_insert_rowid();
            let created_at: String = conn
                .query_row(
                    "SELECT created_at FROM vocabulary_entries WHERE id = ?1",
                    params![vocabulary_entry_id],
                    |row| row.get(0),
                )
                .with_context(|| {
                    format!(
                        "failed to read created_at for vocabulary entry {}",
                        vocabulary_entry_id
                    )
                })?;

            conn.execute(
                "INSERT INTO review_state
                 (vocabulary_entry_id, next_review_at, interval_days, ease_factor, review_count, lapses, state, last_reviewed_at)
                 VALUES (?1, ?2, 0, 2.5, 0, 0, 'new', NULL)",
                params![vocabulary_entry_id, created_at],
            )
            .with_context(|| {
                format!(
                    "failed to insert review_state for vocabulary entry {}",
                    vocabulary_entry_id
                )
            })?;
        }

        Ok(deck_id)
    })();

    match save_result {
        Ok(deck_id) => {
            conn.execute_batch("COMMIT;")
                .context("failed to commit deck save transaction")?;
            Ok(deck_id)
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}

pub fn list_decks(conn: &Connection) -> Result<Vec<DeckSummary>> {
    let mut stmt = conn.prepare(
        "SELECT
            d.id,
            d.title,
            d.source_type,
            d.source_path,
            d.created_at,
            d.sentence_count,
            COUNT(v.id) AS vocabulary_count
         FROM decks d
         LEFT JOIN vocabulary_entries v ON v.deck_id = d.id
         GROUP BY d.id, d.title, d.source_type, d.source_path, d.created_at, d.sentence_count
         ORDER BY d.created_at DESC, d.id DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(DeckSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            source_type: row.get(2)?,
            source_path: row.get(3)?,
            created_at: row.get(4)?,
            sentence_count: row.get(5)?,
            vocabulary_count: row.get(6)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list decks")
}

pub fn get_deck_entries(conn: &Connection, deck_id: i64) -> Result<Vec<VocabularyEntry>> {
    let mut stmt = conn.prepare(
        "SELECT lemma, context, original_token, wordnet_pos
         FROM vocabulary_entries
         WHERE deck_id = ?1
         ORDER BY id ASC",
    )?;

    let rows = stmt.query_map(params![deck_id], |row| {
        let wordnet_pos = row
            .get::<_, Option<String>>(3)?
            .map(|value| wordnet_pos_from_sql(&value))
            .transpose()?;

        Ok(VocabularyEntry {
            lemma: row.get(0)?,
            context: row.get(1)?,
            original_token: row.get(2)?,
            wordnet_pos,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to fetch deck vocabulary entries")
}

pub fn load_study_snapshot(conn: &Connection) -> Result<StudySnapshot> {
    let learned_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM vocabulary_entries", [], |row| row.get(0))
        .context("failed to count vocabulary entries for study snapshot")?;

    let due_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM review_state WHERE next_review_at <= CURRENT_TIMESTAMP",
            [],
            |row| row.get(0),
        )
        .context("failed to count due review items for study snapshot")?;

    Ok(StudySnapshot {
        learned_total,
        streak_days: 0,
        due_today,
    })
}

fn init_connection(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS decks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            source_type TEXT NOT NULL CHECK (source_type IN ('url', 'file')),
            source_path TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            sentence_count INTEGER NOT NULL DEFAULT 0 CHECK (sentence_count >= 0)
        );

        CREATE TABLE IF NOT EXISTS vocabulary_entries (
            id INTEGER PRIMARY KEY,
            deck_id INTEGER NOT NULL,
            lemma TEXT NOT NULL,
            context TEXT NOT NULL,
            original_token TEXT NOT NULL,
            wordnet_pos TEXT CHECK (wordnet_pos IN ('NOUN', 'VERB', 'ADJ', 'ADV')),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (deck_id) REFERENCES decks(id) ON DELETE CASCADE,
            UNIQUE (deck_id, lemma)
        );

        CREATE TABLE IF NOT EXISTS review_state (
            vocabulary_entry_id INTEGER PRIMARY KEY,
            next_review_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            interval_days REAL NOT NULL DEFAULT 0 CHECK (interval_days >= 0),
            ease_factor REAL NOT NULL DEFAULT 2.5 CHECK (ease_factor >= 1.3),
            review_count INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
            lapses INTEGER NOT NULL DEFAULT 0 CHECK (lapses >= 0),
            state TEXT NOT NULL DEFAULT 'new'
                CHECK (state IN ('new', 'learning', 'review', 'relearning', 'suspended')),
            last_reviewed_at TEXT,
            FOREIGN KEY (vocabulary_entry_id) REFERENCES vocabulary_entries(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_decks_created_at
        ON decks(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_vocab_deck_id
        ON vocabulary_entries(deck_id);

        CREATE INDEX IF NOT EXISTS idx_vocab_lemma
        ON vocabulary_entries(lemma);

        CREATE INDEX IF NOT EXISTS idx_review_next_review_at
        ON review_state(next_review_at);

        CREATE INDEX IF NOT EXISTS idx_review_state
        ON review_state(state);
        ",
    )
    .context("failed to initialize SQLite schema")
}

fn validate_source_type(source_type: &str) -> Result<()> {
    if matches!(source_type, "url" | "file") {
        Ok(())
    } else {
        bail!("source_type must be either 'url' or 'file', got '{source_type}'")
    }
}

fn wordnet_pos_to_sql(value: Option<WordnetPos>) -> Option<&'static str> {
    match value {
        Some(WordnetPos::Noun) => Some("NOUN"),
        Some(WordnetPos::Verb) => Some("VERB"),
        Some(WordnetPos::Adj) => Some("ADJ"),
        Some(WordnetPos::Adv) => Some("ADV"),
        None => None,
    }
}

fn wordnet_pos_from_sql(value: &str) -> rusqlite::Result<WordnetPos> {
    match value {
        "NOUN" => Ok(WordnetPos::Noun),
        "VERB" => Ok(WordnetPos::Verb),
        "ADJ" => Ok(WordnetPos::Adj),
        "ADV" => Ok(WordnetPos::Adv),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid wordnet_pos value in database: '{value}'"),
            )),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    fn test_entry(
        lemma: &str,
        context: &str,
        original_token: &str,
        wordnet_pos: Option<WordnetPos>,
    ) -> VocabularyEntry {
        VocabularyEntry {
            lemma: lemma.to_string(),
            context: context.to_string(),
            original_token: original_token.to_string(),
            wordnet_pos,
        }
    }

    fn init_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_connection(&conn).unwrap();
        conn
    }

    #[test]
    fn saves_and_reads_back_deck_data() {
        let conn = init_test_db();
        let entries = vec![
            test_entry(
                "dataset",
                "Researchers are analyzing multilingual datasets for robust tagging.",
                "datasets",
                Some(WordnetPos::Noun),
            ),
            test_entry(
                "gracefully",
                "A clever parser should ignore malformed tokens gracefully.",
                "gracefully",
                Some(WordnetPos::Adv),
            ),
        ];

        let deck_id = save_deck(
            &conn,
            "BBC Technology",
            "url",
            "https://www.bbc.com/news/articles/c70vqwengxno",
            18,
            &entries,
        )
        .unwrap();

        let decks = list_decks(&conn).unwrap();
        assert_eq!(decks.len(), 1);
        assert_eq!(decks[0].id, deck_id);
        assert_eq!(decks[0].title, "BBC Technology");
        assert_eq!(decks[0].source_type, "url");
        assert_eq!(decks[0].sentence_count, 18);
        assert_eq!(decks[0].vocabulary_count, entries.len() as i64);
        assert_ne!(decks[0].sentence_count, decks[0].vocabulary_count);

        let loaded_entries = get_deck_entries(&conn, deck_id).unwrap();
        assert_eq!(loaded_entries, entries);
    }

    #[test]
    fn next_review_matches_vocabulary_entry_created_at() {
        let conn = init_test_db();
        let entries = vec![test_entry(
            "architecture",
            "Our American guide described Parisian architecture to curious visitors.",
            "architecture",
            Some(WordnetPos::Noun),
        )];

        let deck_id = save_deck(
            &conn,
            "Architecture Deck",
            "file",
            "/tmp/architecture.txt",
            1,
            &entries,
        )
        .unwrap();

        let (created_at, next_review_at): (String, String) = conn
            .query_row(
                "SELECT v.created_at, r.next_review_at
                 FROM vocabulary_entries v
                 JOIN review_state r ON r.vocabulary_entry_id = v.id
                 WHERE v.deck_id = ?1",
                params![deck_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(created_at, next_review_at);
    }

    #[test]
    fn duplicate_lemma_in_same_deck_fails_and_rolls_back_transaction() {
        let conn = init_test_db();
        let entries = vec![
            test_entry(
                "dataset",
                "Researchers are analyzing multilingual datasets for robust tagging.",
                "datasets",
                Some(WordnetPos::Noun),
            ),
            test_entry(
                "dataset",
                "Duplicate lemma should fail in the same deck.",
                "dataset",
                Some(WordnetPos::Noun),
            ),
        ];

        let err = save_deck(&conn, "Duplicate Deck", "file", "/tmp/duplicate.txt", 2, &entries)
            .expect_err("duplicate lemma should violate UNIQUE(deck_id, lemma)");

        let message = format!("{err:#}");
        assert!(message.contains("dataset"));

        let deck_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM decks", [], |row| row.get(0))
            .unwrap();
        let vocab_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM vocabulary_entries", [], |row| row.get(0))
            .unwrap();
        let review_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM review_state", [], |row| row.get(0))
            .unwrap();

        assert_eq!(deck_count, 0);
        assert_eq!(vocab_count, 0);
        assert_eq!(review_count, 0);
    }

    #[test]
    fn default_db_path_uses_lexiflash_data_directory() {
        let path = default_db_path().unwrap();
        assert!(path.ends_with("lexiflash/lexiflash.db"));
    }

    #[test]
    fn get_deck_entries_returns_empty_for_unknown_deck() {
        let conn = init_test_db();
        let entries = get_deck_entries(&conn, 999).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn list_decks_is_empty_for_new_database() {
        let conn = init_test_db();
        let decks = list_decks(&conn).unwrap();
        assert!(decks.is_empty());
    }

    #[test]
    fn load_study_snapshot_returns_zeroes_for_new_database() {
        let conn = init_test_db();
        let snapshot = load_study_snapshot(&conn).unwrap();
        assert_eq!(
            snapshot,
            StudySnapshot {
                learned_total: 0,
                streak_days: 0,
                due_today: 0,
            }
        );
    }

    #[test]
    fn init_db_creates_file_on_disk() {
        let temp_dir = std::env::temp_dir().join(format!(
            "lexiflash-db-test-{}",
            std::process::id()
        ));
        let db_path = temp_dir.join("lexiflash.db");
        let conn = init_db(&db_path).unwrap();
        drop(conn);

        assert!(db_path.exists());

        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn save_deck_rejects_invalid_source_type() {
        let conn = init_test_db();
        let entries = vec![test_entry(
            "parser",
            "A clever parser should ignore malformed tokens gracefully.",
            "parser",
            Some(WordnetPos::Noun),
        )];

        let err = save_deck(&conn, "Bad Source", "clipboard", "clipboard://text", 1, &entries)
            .expect_err("invalid source_type must fail");
        assert!(format!("{err:#}").contains("source_type"));
    }

    #[test]
    fn review_state_starts_due_immediately() {
        let conn = init_test_db();
        let entries = vec![test_entry(
            "gracefully",
            "A clever parser should ignore malformed tokens gracefully.",
            "gracefully",
            Some(WordnetPos::Adv),
        )];

        let deck_id = save_deck(&conn, "Due Now", "file", "/tmp/due.txt", 1, &entries).unwrap();
        let (interval_days, ease_factor, review_count, lapses, state, last_reviewed_at): (
            f64,
            f64,
            i64,
            i64,
            String,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT r.interval_days, r.ease_factor, r.review_count, r.lapses, r.state, r.last_reviewed_at
                 FROM review_state r
                 JOIN vocabulary_entries v ON v.id = r.vocabulary_entry_id
                 WHERE v.deck_id = ?1",
                params![deck_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(interval_days, 0.0);
        assert_eq!(ease_factor, 2.5);
        assert_eq!(review_count, 0);
        assert_eq!(lapses, 0);
        assert_eq!(state, "new");
        assert!(last_reviewed_at.is_none());
    }

    #[test]
    fn review_state_exists_for_each_saved_entry() {
        let conn = init_test_db();
        let entries = vec![
            test_entry("dataset", "Dataset context.", "datasets", Some(WordnetPos::Noun)),
            test_entry("gracefully", "Gracefully context.", "gracefully", Some(WordnetPos::Adv)),
        ];

        let deck_id = save_deck(&conn, "Two Entries", "file", "/tmp/two.txt", 4, &entries).unwrap();
        let review_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM review_state r
                 JOIN vocabulary_entries v ON v.id = r.vocabulary_entry_id
                 WHERE v.deck_id = ?1",
                params![deck_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(review_count, 2);
    }

    #[test]
    fn load_study_snapshot_counts_saved_entries_and_due_today() {
        let conn = init_test_db();
        let entries = vec![
            test_entry("dataset", "Dataset context.", "datasets", Some(WordnetPos::Noun)),
            test_entry("gracefully", "Gracefully context.", "gracefully", Some(WordnetPos::Adv)),
        ];

        let deck_id = save_deck(&conn, "Snapshot Deck", "file", "/tmp/snapshot.txt", 3, &entries)
            .unwrap();

        conn.execute(
            "UPDATE review_state
             SET next_review_at = datetime('now', '+1 day')
             WHERE vocabulary_entry_id = (
                 SELECT id FROM vocabulary_entries
                 WHERE deck_id = ?1
                 ORDER BY id DESC
                 LIMIT 1
             )",
            params![deck_id],
        )
        .unwrap();

        let snapshot = load_study_snapshot(&conn).unwrap();
        assert_eq!(snapshot.learned_total, 2);
        assert_eq!(snapshot.streak_days, 0);
        assert_eq!(snapshot.due_today, 1);
    }

    #[test]
    fn can_find_saved_deck_with_optional_review_query() {
        let conn = init_test_db();
        let entries = vec![test_entry("parser", "Parser context.", "parser", Some(WordnetPos::Noun))];
        let deck_id = save_deck(&conn, "Lookup", "file", "/tmp/lookup.txt", 1, &entries).unwrap();

        let found: Option<String> = conn
            .query_row(
                "SELECT title FROM decks WHERE id = ?1",
                params![deck_id],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(found.as_deref(), Some("Lookup"));
    }
}
