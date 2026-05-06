//! plan_ref:
//!   - 14_tech_stack#search-baseline

use std::cmp::Ordering;

pub(super) fn score_desc(left: f32, right: f32) -> Ordering {
    sortable_score(right).total_cmp(&sortable_score(left))
}

fn sortable_score(score: f32) -> f32 {
    if score.is_nan() {
        f32::NEG_INFINITY
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::score_desc;

    #[test]
    fn score_desc_sorts_nan_last_without_panicking() {
        let mut scores = [0.5, f32::NAN, 2.0];

        scores.sort_by(|left, right| score_desc(*left, *right));

        assert_eq!(scores[0], 2.0);
        assert_eq!(scores[1], 0.5);
        assert!(scores[2].is_nan());
    }
}
