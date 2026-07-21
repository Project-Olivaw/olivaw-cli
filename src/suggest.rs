//! Did-you-mean suggestions for mistyped component and category names.
//! Tiny two-row Levenshtein — not worth a dependency.

/// Edit distance between `a` and `b` (case-sensitive; callers lowercase first).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// The closest candidate to `input`, if it is close enough to be a plausible
/// typo (distance ≤ 2, or ≤ a third of the input length for long names).
pub fn did_you_mean<'a, I>(input: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let threshold = (input.len() / 3).max(2);
    candidates
        .into_iter()
        .map(|c| (levenshtein(input, c), c))
        .filter(|(d, _)| *d <= threshold)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_basics() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("mpu650", "mpu6050"), 1);
    }

    #[test]
    fn suggests_close_names() {
        let names = ["sensors/mpu6050", "sensors/hcsr04", "drivers/l298n"];
        assert_eq!(
            did_you_mean("sensors/mpu650", names),
            Some("sensors/mpu6050")
        );
        assert_eq!(did_you_mean("drivers/l298", names), Some("drivers/l298n"));
    }

    #[test]
    fn stays_quiet_when_nothing_is_close() {
        let names = ["sensors/mpu6050", "drivers/l298n"];
        assert_eq!(did_you_mean("slam/pose-graph", names), None);
    }
}
