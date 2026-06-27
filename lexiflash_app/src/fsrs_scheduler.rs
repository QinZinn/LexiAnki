use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};
use fsrs::{FSRS, MemoryState};

pub const DESIRED_RETENTION: f32 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewRating {
    Again,
    Hard,
    Good,
    Easy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStateKind {
    New,
    Learning,
    Review,
    Relearning,
    Suspended,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewStateRecord {
    pub vocabulary_entry_id: i64,
    pub due_at: DateTime<Utc>,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub reps: i64,
    pub lapses: i64,
    pub step: Option<i64>,
    pub state: ReviewStateKind,
    pub last_review_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledReviewState {
    pub due_at: DateTime<Utc>,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub step: Option<i64>,
    pub state: ReviewStateKind,
    pub last_review_at: DateTime<Utc>,
    pub interval_days: i64,
}

pub fn schedule_review(
    current: &ReviewStateRecord,
    rating: ReviewRating,
) -> Result<ScheduledReviewState> {
    schedule_review_at(current, rating, Utc::now())
}

pub fn schedule_review_at(
    current: &ReviewStateRecord,
    rating: ReviewRating,
    now: DateTime<Utc>,
) -> Result<ScheduledReviewState> {
    if matches!(current.state, ReviewStateKind::Suspended) {
        bail!("cannot schedule suspended card {}", current.vocabulary_entry_id);
    }

    let anchor = current.last_review_at.unwrap_or(current.due_at);
    let days_elapsed = now.signed_duration_since(anchor).num_days().max(0) as u32;
    let fsrs = FSRS::default();
    let next_states = fsrs.next_states(current.memory_state(), DESIRED_RETENTION, days_elapsed)?;
    let selected = match rating {
        ReviewRating::Again => next_states.again,
        ReviewRating::Hard => next_states.hard,
        ReviewRating::Good => next_states.good,
        ReviewRating::Easy => next_states.easy,
    };

    let interval_days = selected.interval.round().max(1.0) as i64;
    let due_at = now + Duration::days(interval_days);
    let (state, step) = transition_state(current.state, current.step, rating);

    Ok(ScheduledReviewState {
        due_at,
        stability: f64::from(selected.memory.stability),
        difficulty: f64::from(selected.memory.difficulty),
        reps: current.reps + 1,
        lapses: current.lapses + i64::from(matches!(rating, ReviewRating::Again)),
        step,
        state,
        last_review_at: now,
        interval_days,
    })
}

impl ReviewStateRecord {
    pub fn memory_state(&self) -> Option<MemoryState> {
        match (self.stability, self.difficulty) {
            (Some(stability), Some(difficulty)) => Some(MemoryState {
                stability: stability as f32,
                difficulty: difficulty as f32,
            }),
            _ => None,
        }
    }
}

fn transition_state(
    current: ReviewStateKind,
    current_step: Option<i64>,
    rating: ReviewRating,
) -> (ReviewStateKind, Option<i64>) {
    match current {
        ReviewStateKind::New => match rating {
            ReviewRating::Again | ReviewRating::Hard => (ReviewStateKind::Learning, Some(0)),
            ReviewRating::Good | ReviewRating::Easy => (ReviewStateKind::Review, None),
        },
        ReviewStateKind::Learning => match rating {
            ReviewRating::Again | ReviewRating::Hard => {
                (ReviewStateKind::Learning, Some(current_step.unwrap_or(0) + 1))
            }
            ReviewRating::Good | ReviewRating::Easy => (ReviewStateKind::Review, None),
        },
        ReviewStateKind::Review => match rating {
            ReviewRating::Again => (ReviewStateKind::Relearning, Some(0)),
            ReviewRating::Hard | ReviewRating::Good | ReviewRating::Easy => {
                (ReviewStateKind::Review, None)
            }
        },
        ReviewStateKind::Relearning => match rating {
            ReviewRating::Again | ReviewRating::Hard => {
                (ReviewStateKind::Relearning, Some(current_step.unwrap_or(0) + 1))
            }
            ReviewRating::Good | ReviewRating::Easy => (ReviewStateKind::Review, None),
        },
        ReviewStateKind::Suspended => (ReviewStateKind::Suspended, current_step),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn new_card(now: DateTime<Utc>) -> ReviewStateRecord {
        ReviewStateRecord {
            vocabulary_entry_id: 42,
            due_at: now,
            stability: None,
            difficulty: None,
            reps: 0,
            lapses: 0,
            step: None,
            state: ReviewStateKind::New,
            last_review_at: None,
        }
    }

    #[test]
    fn first_good_review_schedules_card_into_future() {
        let now = Utc.with_ymd_and_hms(2026, 6, 27, 12, 0, 0).unwrap();
        let card = new_card(now);

        let scheduled = schedule_review_at(&card, ReviewRating::Good, now).unwrap();

        assert!(scheduled.due_at > now);
        assert!(scheduled.interval_days >= 1);
        assert!(matches!(
            scheduled.state,
            ReviewStateKind::Learning | ReviewStateKind::Review
        ));
        assert_eq!(scheduled.state, ReviewStateKind::Review);
        assert_eq!(scheduled.reps, 1);
        assert_eq!(scheduled.lapses, 0);
        assert_eq!(scheduled.last_review_at, now);
    }

    #[test]
    fn again_increases_lapses_and_schedules_closer_than_easy() {
        let now = Utc.with_ymd_and_hms(2026, 6, 27, 12, 0, 0).unwrap();
        let card = new_card(now);

        let again = schedule_review_at(&card, ReviewRating::Again, now).unwrap();
        let good = schedule_review_at(&card, ReviewRating::Good, now).unwrap();
        let easy = schedule_review_at(&card, ReviewRating::Easy, now).unwrap();

        assert_eq!(again.lapses, 1);
        assert_eq!(good.lapses, 0);
        assert_eq!(easy.lapses, 0);
        assert!(again.interval_days <= good.interval_days);
        assert!(good.interval_days <= easy.interval_days);
        assert_eq!(again.state, ReviewStateKind::Learning);
        assert_eq!(easy.state, ReviewStateKind::Review);
    }
}
