#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comparison {
    pub reference_median_ns: f64,
    pub evolution_median_ns: f64,
    pub performance_ratio: f64,
    pub correctness: bool,
    pub stable_measurement: bool,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareError {
    EmptyReferenceSamples,
    EmptyEvolutionSamples,
    ZeroReferenceMedian,
}

pub fn compare_samples(
    reference_samples_ns: &[u128],
    evolution_samples_ns: &[u128],
    correctness: bool,
    stable_measurement: bool,
) -> Result<Comparison, CompareError> {
    let reference_median_ns = median(reference_samples_ns)
        .ok_or(CompareError::EmptyReferenceSamples)?;
    let evolution_median_ns = median(evolution_samples_ns)
        .ok_or(CompareError::EmptyEvolutionSamples)?;
    if reference_median_ns == 0.0 {
        return Err(CompareError::ZeroReferenceMedian);
    }

    let performance_ratio = evolution_median_ns / reference_median_ns;
    let verdict = classify(correctness, stable_measurement, performance_ratio);

    Ok(Comparison {
        reference_median_ns,
        evolution_median_ns,
        performance_ratio,
        correctness,
        stable_measurement,
        verdict,
    })
}

#[must_use]
pub fn classify(correctness: bool, stable_measurement: bool, performance_ratio: f64) -> Verdict {
    if !correctness {
        return Verdict::Fail;
    }
    if !stable_measurement || !performance_ratio.is_finite() {
        return Verdict::Inconclusive;
    }
    if performance_ratio <= 1.0 {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

fn median(samples: &[u128]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut values = samples.to_vec();
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[middle] as f64)
    } else {
        Some((values[middle - 1] as f64 + values[middle] as f64) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CompareError, Verdict, classify, compare_samples};

    #[test]
    fn parity_is_a_pass() {
        let result = compare_samples(&[100, 100, 100], &[100, 100, 100], true, true)
            .expect("comparison should succeed");
        assert_eq!(result.performance_ratio, 1.0);
        assert_eq!(result.verdict, Verdict::Pass);
    }

    #[test]
    fn faster_is_a_pass() {
        assert_eq!(classify(true, true, 0.95), Verdict::Pass);
    }

    #[test]
    fn repeatable_regression_is_a_fail() {
        assert_eq!(classify(true, true, 1.000_001), Verdict::Fail);
    }

    #[test]
    fn incorrect_output_is_always_a_fail() {
        assert_eq!(classify(false, true, 0.5), Verdict::Fail);
    }

    #[test]
    fn unstable_measurement_is_inconclusive() {
        assert_eq!(classify(true, false, 0.9), Verdict::Inconclusive);
    }

    #[test]
    fn empty_samples_are_rejected() {
        assert_eq!(
            compare_samples(&[], &[1], true, true),
            Err(CompareError::EmptyReferenceSamples)
        );
    }
}
