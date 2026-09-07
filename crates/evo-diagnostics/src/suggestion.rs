const MAX_EDIT_DISTANCE: usize = 1;

/// Returns one conservative spelling suggestion from a context-specific candidate set.
///
/// Evolution identifiers are currently ASCII. v0 therefore accepts only one insertion,
/// deletion, substitution, or adjacent transposition. Equal-best candidates are deliberately
/// suppressed instead of guessing.
#[must_use]
pub fn best_suggestion<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    if input.len() < 2 {
        return None;
    }

    let mut candidates: Vec<&str> = candidates
        .into_iter()
        .filter(|candidate| candidate.len() >= 2 && *candidate != input)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();

    let mut best = None;
    for candidate in candidates {
        let Some(distance) = edit_distance_with_limit(input, candidate, MAX_EDIT_DISTANCE) else {
            continue;
        };
        debug_assert!(distance > 0);

        match &best {
            None => best = Some((distance, candidate)),
            Some((best_distance, _)) if distance < *best_distance => {
                best = Some((distance, candidate));
            }
            Some((best_distance, _)) if distance == *best_distance => return None,
            Some(_) => {}
        }
    }

    best.map(|(_, candidate)| candidate.to_owned())
}

fn edit_distance_with_limit(left: &str, right: &str, limit: usize) -> Option<usize> {
    debug_assert_eq!(limit, 1, "v0 matcher is intentionally a one-edit matcher");
    if left == right {
        return Some(0);
    }

    let left = left.as_bytes();
    let right = right.as_bytes();
    let length_difference = left.len().abs_diff(right.len());
    if length_difference > limit {
        return None;
    }

    if left.len() == right.len() {
        let mismatches: Vec<usize> = left
            .iter()
            .zip(right)
            .enumerate()
            .filter_map(|(index, (left, right))| (left != right).then_some(index))
            .collect();
        return match mismatches.as_slice() {
            [_] => Some(1),
            [first, second]
                if *second == *first + 1
                    && left[*first] == right[*second]
                    && left[*second] == right[*first] =>
            {
                Some(1)
            }
            _ => None,
        };
    }

    let (shorter, longer) = if left.len() < right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let mut short_index = 0;
    let mut long_index = 0;
    let mut skipped = false;

    while short_index < shorter.len() && long_index < longer.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
            continue;
        }
        if skipped {
            return None;
        }
        skipped = true;
        long_index += 1;
    }

    Some(1)
}

#[cfg(test)]
mod tests {
    use super::best_suggestion;

    #[test]
    fn suggests_single_insert_delete_substitute_or_transpose() {
        assert_eq!(
            best_suggestion("coun", ["count"].into_iter()),
            Some("count".to_owned())
        );
        assert_eq!(
            best_suggestion("countt", ["count"].into_iter()),
            Some("count".to_owned())
        );
        assert_eq!(
            best_suggestion("coumt", ["count"].into_iter()),
            Some("count".to_owned())
        );
        assert_eq!(
            best_suggestion("coutn", ["count"].into_iter()),
            Some("count".to_owned())
        );
    }

    #[test]
    fn suppresses_ties_distant_names_and_single_character_guesses() {
        assert_eq!(best_suggestion("cot", ["cat", "cut"].into_iter()), None);
        assert_eq!(best_suggestion("counter", ["value"].into_iter()), None);
        assert_eq!(best_suggestion("x", ["y"].into_iter()), None);
    }

    #[test]
    fn candidate_order_and_duplicates_do_not_change_the_result() {
        assert_eq!(
            best_suggestion("coutn", ["other", "count", "count"].into_iter()),
            Some("count".to_owned())
        );
        assert_eq!(
            best_suggestion("coutn", ["count", "other"].into_iter()),
            Some("count".to_owned())
        );
    }
}
