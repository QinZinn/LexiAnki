use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use lexianki_nlp::{VocabularyEntry, WordnetPos};
use rusqlite::{Connection, params};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::fsrs_scheduler::{ReviewStateKind, ReviewStateRecord, ScheduledReviewState};

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
                 (vocabulary_entry_id, due_at, stability, difficulty, reps, lapses, step, state, last_review_at)
                 VALUES (?1, ?2, NULL, NULL, 0, 0, NULL, 'new', NULL)",
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
            "SELECT COUNT(*) FROM review_state WHERE due_at <= CURRENT_TIMESTAMP",
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

pub fn update_review_state(
    conn: &Connection,
    vocabulary_entry_id: i64,
    new_state: &ScheduledReviewState,
) -> Result<()> {
    conn.execute(
        "UPDATE review_state
         SET due_at = ?2,
             stability = ?3,
             difficulty = ?4,
             reps = ?5,
             lapses = ?6,
             step = ?7,
             state = ?8,
             last_review_at = ?9
         WHERE vocabulary_entry_id = ?1",
        params![
            vocabulary_entry_id,
            format_db_timestamp(new_state.due_at),
            new_state.stability,
            new_state.difficulty,
            new_state.reps,
            new_state.lapses,
            new_state.step,
            review_state_kind_to_sql(new_state.state),
            format_db_timestamp(new_state.last_review_at),
        ],
    )
    .with_context(|| format!("failed to update review_state for vocabulary entry {vocabulary_entry_id}"))?;

    Ok(())
}

pub fn get_review_state(conn: &Connection, vocabulary_entry_id: i64) -> Result<ReviewStateRecord> {
    conn.query_row(
        "SELECT vocabulary_entry_id, due_at, stability, difficulty, reps, lapses, step, state, last_review_at
         FROM review_state
         WHERE vocabulary_entry_id = ?1",
        params![vocabulary_entry_id],
        review_state_from_row,
    )
    .with_context(|| format!("failed to fetch review_state for vocabulary entry {vocabulary_entry_id}"))
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

        CREATE INDEX IF NOT EXISTS idx_decks_created_at
        ON decks(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_vocab_deck_id
        ON vocabulary_entries(deck_id);

        CREATE INDEX IF NOT EXISTS idx_vocab_lemma
        ON vocabulary_entries(lemma);
        ",
    )
    .context("failed to initialize base SQLite schema")?;

    ensure_review_state_schema(conn)?;
    Ok(())
}

fn ensure_review_state_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("DROP INDEX IF EXISTS idx_review_next_review_at;")
        .context("failed to remove legacy review_state index")?;

    if review_state_needs_rebuild(conn)? {
        conn.execute_batch(
            "
            DROP INDEX IF EXISTS idx_review_due_at;
            DROP INDEX IF EXISTS idx_review_state;
            DROP TABLE IF EXISTS review_state;
            ",
        )
        .context("failed to drop legacy review_state schema")?;
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS review_state (
            vocabulary_entry_id INTEGER PRIMARY KEY,
            due_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            stability REAL CHECK (stability IS NULL OR stability > 0),
            difficulty REAL CHECK (
                difficulty IS NULL OR (difficulty >= 1.0 AND difficulty <= 10.0)
            ),
            reps INTEGER NOT NULL DEFAULT 0 CHECK (reps >= 0),
            lapses INTEGER NOT NULL DEFAULT 0 CHECK (lapses >= 0),
            step INTEGER CHECK (step IS NULL OR step >= 0),
            state TEXT NOT NULL DEFAULT 'new'
                CHECK (state IN ('new', 'learning', 'review', 'relearning', 'suspended')),
            last_review_at TEXT,
            FOREIGN KEY (vocabulary_entry_id) REFERENCES vocabulary_entries(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_review_due_at
        ON review_state(due_at);

        CREATE INDEX IF NOT EXISTS idx_review_state
        ON review_state(state);
        ",
    )
    .context("failed to initialize FSRS review_state schema")
}

fn review_state_needs_rebuild(conn: &Connection) -> Result<bool> {
    let review_state_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'review_state'",
            [],
            |row| row.get(0),
        )
        .context("failed to inspect review_state table presence")?;

    if review_state_exists == 0 {
        return Ok(false);
    }

    let mut stmt = conn
        .prepare("PRAGMA table_info(review_state)")
        .context("failed to inspect review_state columns")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .context("failed to query review_state columns")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to collect review_state columns")?;

    let has_legacy_columns = ["next_review_at", "interval_days", "ease_factor", "review_count", "last_reviewed_at"]
        .iter()
        .any(|column| columns.iter().any(|value| value == column));
    let has_fsrs_columns = [
        "due_at",
        "stability",
        "difficulty",
        "reps",
        "lapses",
        "step",
        "state",
        "last_review_at",
    ]
    .iter()
    .all(|column| columns.iter().any(|value| value == column));

    Ok(has_legacy_columns || !has_fsrs_columns)
}

fn validate_source_type(source_type: &str) -> Result<()> {
    if matches!(source_type, "url" | "file") {
        Ok(())
    } else {
        bail!("source_type must be either 'url' or 'file', got '{source_type}'")
    }
}

fn parse_db_timestamp(value: &str) -> io::Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid SQLite timestamp '{value}': {err}"),
        )
    })?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn format_db_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn review_state_kind_to_sql(value: ReviewStateKind) -> &'static str {
    match value {
        ReviewStateKind::New => "new",
        ReviewStateKind::Learning => "learning",
        ReviewStateKind::Review => "review",
        ReviewStateKind::Relearning => "relearning",
        ReviewStateKind::Suspended => "suspended",
    }
}

fn review_state_kind_from_sql(value: &str) -> rusqlite::Result<ReviewStateKind> {
    match value {
        "new" => Ok(ReviewStateKind::New),
        "learning" => Ok(ReviewStateKind::Learning),
        "review" => Ok(ReviewStateKind::Review),
        "relearning" => Ok(ReviewStateKind::Relearning),
        "suspended" => Ok(ReviewStateKind::Suspended),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid review state value in database: '{value}'"),
            )),
        )),
    }
}

fn review_state_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReviewStateRecord> {
    let due_at_raw: String = row.get(1)?;
    let last_review_raw: Option<String> = row.get(8)?;

    let due_at = parse_db_timestamp(&due_at_raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let last_review_at = last_review_raw
        .as_deref()
        .map(parse_db_timestamp)
        .transpose()
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                8,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })?;

    Ok(ReviewStateRecord {
        vocabulary_entry_id: row.get(0)?,
        due_at,
        stability: row.get(2)?,
        difficulty: row.get(3)?,
        reps: row.get(4)?,
        lapses: row.get(5)?,
        step: row.get(6)?,
        state: review_state_kind_from_sql(&row.get::<_, String>(7)?)?,
        last_review_at,
    })
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
    use crate::fsrs_scheduler::{ReviewStateKind, ScheduledReviewState};
    use chrono::TimeZone;
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
    fn due_at_matches_vocabulary_entry_created_at() {
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

        let (created_at, due_at): (String, String) = conn
            .query_row(
                "SELECT v.created_at, r.due_at
                 FROM vocabulary_entries v
                 JOIN review_state r ON r.vocabulary_entry_id = v.id
                 WHERE v.deck_id = ?1",
                params![deck_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(created_at, due_at);
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
        let (
            due_at,
            created_at,
            stability,
            difficulty,
            reps,
            lapses,
            step,
            state,
            last_review_at,
        ): (
            String,
            String,
            Option<f64>,
            Option<f64>,
            i64,
            i64,
            Option<i64>,
            String,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT r.due_at, v.created_at, r.stability, r.difficulty, r.reps, r.lapses, r.step, r.state, r.last_review_at
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
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(due_at, created_at);
        assert!(stability.is_none());
        assert!(difficulty.is_none());
        assert_eq!(reps, 0);
        assert_eq!(lapses, 0);
        assert!(step.is_none());
        assert_eq!(state, "new");
        assert!(last_review_at.is_none());
    }

    #[test]
    fn update_review_state_persists_fsrs_values() {
        let conn = init_test_db();
        let entries = vec![test_entry(
            "dataset",
            "Researchers are analyzing multilingual datasets for robust tagging.",
            "datasets",
            Some(WordnetPos::Noun),
        )];
        let deck_id = save_deck(&conn, "FSRS Deck", "file", "/tmp/fsrs.txt", 1, &entries).unwrap();
        let vocabulary_entry_id: i64 = conn
            .query_row(
                "SELECT id FROM vocabulary_entries WHERE deck_id = ?1 LIMIT 1",
                params![deck_id],
                |row| row.get(0),
            )
            .unwrap();

        let due_at = Utc.with_ymd_and_hms(2026, 6, 30, 9, 0, 0).unwrap();
        let reviewed_at = Utc.with_ymd_and_hms(2026, 6, 27, 9, 0, 0).unwrap();
        let scheduled = ScheduledReviewState {
            due_at,
            stability: 3.25,
            difficulty: 5.75,
            reps: 1,
            lapses: 1,
            step: Some(0),
            state: ReviewStateKind::Relearning,
            last_review_at: reviewed_at,
            interval_days: 3,
        };

        update_review_state(&conn, vocabulary_entry_id, &scheduled).unwrap();
        let loaded = get_review_state(&conn, vocabulary_entry_id).unwrap();

        assert_eq!(loaded.vocabulary_entry_id, vocabulary_entry_id);
        assert_eq!(loaded.due_at, due_at);
        assert_eq!(loaded.stability, Some(3.25));
        assert_eq!(loaded.difficulty, Some(5.75));
        assert_eq!(loaded.reps, 1);
        assert_eq!(loaded.lapses, 1);
        assert_eq!(loaded.step, Some(0));
        assert_eq!(loaded.state, ReviewStateKind::Relearning);
        assert_eq!(loaded.last_review_at, Some(reviewed_at));
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
             SET due_at = datetime('now', '+1 day')
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
    fn init_connection_rebuilds_legacy_review_state_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE review_state (
                vocabulary_entry_id INTEGER PRIMARY KEY,
                next_review_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                interval_days REAL NOT NULL DEFAULT 0 CHECK (interval_days >= 0),
                ease_factor REAL NOT NULL DEFAULT 2.5 CHECK (ease_factor >= 1.3),
                review_count INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
                lapses INTEGER NOT NULL DEFAULT 0 CHECK (lapses >= 0),
                state TEXT NOT NULL DEFAULT 'new',
                last_reviewed_at TEXT
            );

            CREATE INDEX idx_review_next_review_at ON review_state(next_review_at);
            ",
        )
        .unwrap();

        init_connection(&conn).unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(review_state)").unwrap();
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();

        for expected in [
            "due_at",
            "stability",
            "difficulty",
            "reps",
            "lapses",
            "step",
            "state",
            "last_review_at",
        ] {
            assert!(columns.iter().any(|column| column == expected));
        }

        for legacy in ["next_review_at", "interval_days", "ease_factor", "review_count", "last_reviewed_at"] {
            assert!(!columns.iter().any(|column| column == legacy));
        }
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
