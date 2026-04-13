use crate::models::SrsReview;
use chrono::{Duration, Utc};

/// SM-2 Algorithm implementation for spaced repetition
/// Based on the SuperMemo SM-2 algorithm
pub struct Sm2Algorithm;

impl Sm2Algorithm {
    /// Calculate next review interval based on quality of recall
    /// quality: 0-5 (0=complete blackout, 5=perfect response)
    pub fn calculate_next_review(
        easiness_factor: f64,
        interval_days: i32,
        repetitions: i32,
        quality: i32,
    ) -> (f64, i32, i32) {
        let mut ef = easiness_factor;
        let mut interval = interval_days;
        let mut reps = repetitions;

        if quality >= 3 {
            // Correct response
            match reps {
                0 => interval = 1,
                1 => interval = 6,
                _ => interval = (interval as f64 * ef).round() as i32,
            }
            reps += 1;
        } else {
            // Incorrect response - reset
            reps = 0;
            interval = 1;
        }

        // Update easiness factor
        ef = ef + (0.1 - (5 - quality) as f64 * (0.08 + (5 - quality) as f64 * 0.02));
        if ef < 1.3 {
            ef = 1.3;
        }

        (ef, interval, reps)
    }
}

/// Leitner System implementation
/// Cards move between boxes based on recall quality
/// Box 1: Every day, Box 2: Every 3 days, Box 3: Every 7 days, Box 4: Every 14 days, Box 5: Every 30 days
pub struct LeitnerSystem;

impl LeitnerSystem {
    pub fn get_review_interval_days(leitner_box: i32) -> i32 {
        match leitner_box {
            1 => 1,    // Daily
            2 => 3,    // Every 3 days
            3 => 7,    // Weekly
            4 => 14,   // Bi-weekly
            5 => 30,   // Monthly
            _ => 30,   // Cap at monthly
        }
    }

    pub fn update_leitner_box(current_box: i32, quality: i32) -> i32 {
        if quality >= 3 {
            // Correct - move up one box (max 5)
            (current_box + 1).min(5)
        } else {
            // Incorrect - reset to box 1
            1
        }
    }

    pub fn get_cards_due_for_review(
        cards: &[SrsReview],
    ) -> Vec<SrsReview> {
        let now = Utc::now();
        cards
            .iter()
            .filter(|card| card.next_review <= now)
            .cloned()
            .collect()
    }
}

/// Hybrid SRS combining SM-2 and Leitner
pub struct HybridSrs;

impl HybridSrs {
    pub fn process_review(
        review: &SrsReview,
        quality: i32,
    ) -> (f64, i32, i32, i32, chrono::DateTime<Utc>) {
        // SM-2 calculation
        let (ef, interval, reps) = Sm2Algorithm::calculate_next_review(
            review.easiness_factor,
            review.interval_days,
            review.repetitions,
            quality,
        );

        // Leitner calculation
        let new_leitner_box = LeitnerSystem::update_leitner_box(review.leitner_box, quality);
        let leitner_interval = LeitnerSystem::get_review_interval_days(new_leitner_box);

        // Use the shorter interval from SM-2 vs Leitner for optimal spacing
        let final_interval = interval.min(leitner_interval);
        let next_review = Utc::now() + Duration::days(final_interval as i64);

        (ef, final_interval, reps, new_leitner_box, next_review)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm2_perfect_recall() {
        let (ef, interval, reps) = Sm2Algorithm::calculate_next_review(2.5, 0, 0, 5);
        assert_eq!(reps, 1);
        assert_eq!(interval, 1);
        assert!(ef > 2.5);
    }

    #[test]
    fn test_sm2_failed_recall() {
        let (ef, interval, reps) = Sm2Algorithm::calculate_next_review(2.5, 6, 2, 1);
        assert_eq!(reps, 0);
        assert_eq!(interval, 1);
        assert!(ef < 2.5);
    }

    #[test]
    fn test_leitner_progression() {
        assert_eq!(LeitnerSystem::update_leitner_box(1, 5), 2);
        assert_eq!(LeitnerSystem::update_leitner_box(4, 4), 5);
        assert_eq!(LeitnerSystem::update_leitner_box(5, 5), 5); // Cap at 5
        assert_eq!(LeitnerSystem::update_leitner_box(3, 2), 1); // Reset on fail
    }
}
