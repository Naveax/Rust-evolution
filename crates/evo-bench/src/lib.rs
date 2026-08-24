use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => f.write_str("PASS"),
            Self::Fail => f.write_str("FAIL"),
            Self::Inconclusive => f.write_str("INCONCLUSIVE"),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleStats {
    pub count: usize,
    pub min_ns: u128,
    pub max_ns: u128,
    pub median_ns: f64,
    pub p95_ns: u128,
    pub relative_mad: f64,
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
    let reference_median_ns =
        median(reference_samples_ns).ok_or(CompareError::EmptyReferenceSamples)?;
    let evolution_median_ns =
        median(evolution_samples_ns).ok_or(CompareError::EmptyEvolutionSamples)?;
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

#[must_use]
pub fn summarize(samples: &[u128]) -> Option<SampleStats> {
    if samples.is_empty() {
        return None;
    }

    let mut values = samples.to_vec();
    values.sort_unstable();
    let median_ns = median_sorted(&values);
    let rank = (values.len() * 95).div_ceil(100);
    let p95_ns = values[rank.saturating_sub(1)];

    let mut deviations: Vec<f64> = values
        .iter()
        .map(|value| ((*value as f64) - median_ns).abs())
        .collect();
    deviations.sort_by(f64::total_cmp);
    let mad = median_sorted_f64(&deviations);
    let relative_mad = if median_ns == 0.0 {
        f64::INFINITY
    } else {
        mad / median_ns
    };

    Some(SampleStats {
        count: values.len(),
        min_ns: values[0],
        max_ns: *values.last().expect("non-empty values"),
        median_ns,
        p95_ns,
        relative_mad,
    })
}

#[must_use]
pub fn measurement_is_stable(
    reference: SampleStats,
    evolution: SampleStats,
    max_relative_mad: f64,
) -> bool {
    max_relative_mad.is_finite()
        && max_relative_mad >= 0.0
        && reference.relative_mad <= max_relative_mad
        && evolution.relative_mad <= max_relative_mad
}

fn median(samples: &[u128]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut values = samples.to_vec();
    values.sort_unstable();
    Some(median_sorted(&values))
}

fn median_sorted(values: &[u128]) -> f64 {
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle] as f64
    } else {
        (values[middle - 1] as f64 + values[middle] as f64) / 2.0
    }
}

fn median_sorted_f64(values: &[f64]) -> f64 {
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompareError, Verdict, classify, compare_samples, measurement_is_stable, summarize,
    };

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

    #[test]
    fn summary_reports_median_p95_and_mad() {
        let stats = summarize(&[100, 110, 90, 100, 105]).expect("summary should exist");
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min_ns, 90);
        assert_eq!(stats.max_ns, 110);
        assert_eq!(stats.median_ns, 100.0);
        assert_eq!(stats.p95_ns, 110);
        assert_eq!(stats.relative_mad, 0.05);
    }

    #[test]
    fn stability_requires_both_sides_under_noise_limit() {
        let quiet = summarize(&[100, 100, 101, 99, 100]).expect("summary should exist");
        let noisy = summarize(&[100, 150, 50, 200, 25]).expect("summary should exist");
        assert!(measurement_is_stable(quiet, quiet, 0.05));
        assert!(!measurement_is_stable(quiet, noisy, 0.05));
    }
}
