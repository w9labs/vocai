use chrono::{Duration, Utc};

/// SM-2 Algorithm implementation for spaced repetition
pub struct Sm2Algorithm;

impl Sm2Algorithm {
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
            match reps {
                0 => interval = 1,
                1 => interval = 6,
                _ => interval = (interval as f64 * ef).round() as i32,
            }
            reps += 1;
        } else {
            reps = 0;
            interval = 1;
        }

        ef = ef + (0.1 - (5 - quality) as f64 * (0.08 + (5 - quality) as f64 * 0.02));
        if ef < 1.3 {
            ef = 1.3;
        }

        (ef, interval, reps)
    }
}

/// Leitner System: Box 1=Daily, Box 2=3d, Box 3=Weekly, Box 4=Biweekly, Box 5=Monthly
pub struct LeitnerSystem;

impl LeitnerSystem {
    pub fn get_review_interval_days(leitner_box: i32) -> i32 {
        match leitner_box {
            1 => 1,
            2 => 3,
            3 => 7,
            4 => 14,
            5 | _ => 30,
        }
    }

    pub fn update_leitner_box(current_box: i32, quality: i32) -> i32 {
        if quality >= 3 {
            (current_box + 1).min(5)
        } else {
            1
        }
    }
}

/// Hybrid SRS combining SM-2 and Leitner
pub struct HybridSrs;

impl HybridSrs {
    /// Process review with explicit SRS parameters (used by review endpoint)
    pub fn process_review_quality(
        easiness_factor: f64,
        interval_days: i32,
        repetitions: i32,
        leitner_box: i32,
        quality: i32,
    ) -> (f64, i32, i32, i32, chrono::DateTime<Utc>) {
        let (ef, interval, reps) = Sm2Algorithm::calculate_next_review(
            easiness_factor,
            interval_days,
            repetitions,
            quality,
        );
        let new_leitner_box = LeitnerSystem::update_leitner_box(leitner_box, quality);
        let leitner_interval = LeitnerSystem::get_review_interval_days(new_leitner_box);
        let final_interval = interval.min(leitner_interval);
        let next_review = Utc::now() + Duration::days(final_interval as i64);
        (ef, final_interval, reps, new_leitner_box, next_review)
    }
}
