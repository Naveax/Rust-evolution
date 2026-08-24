use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct CaseConfig {
    pub(crate) name: String,
    pub(crate) warmup: usize,
    pub(crate) samples: usize,
    pub(crate) timeout: Duration,
    pub(crate) max_relative_mad: f64,
}

impl CaseConfig {
    pub(crate) fn load(case_dir: &Path) -> Result<Self, String> {
        let path = case_dir.join("case.conf");
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        let mut name = None;
        let mut warmup = None;
        let mut samples = None;
        let mut timeout_ms = None;
        let mut max_relative_mad = None;
        let mut seen = HashSet::new();

        for (index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line.split_once('=').ok_or_else(|| {
                format!("{}:{}: expected key=value", path.display(), index + 1)
            })?;
            let key = key.trim();
            let value = value.trim();

            if !seen.insert(key.to_owned()) {
                return Err(format!(
                    "{}:{}: duplicate config key {key:?}",
                    path.display(),
                    index + 1
                ));
            }

            match key {
                "name" => name = Some(value.to_owned()),
                "warmup" => warmup = Some(parse_usize(&path, index, key, value)?),
                "samples" => samples = Some(parse_usize(&path, index, key, value)?),
                "timeout_ms" => timeout_ms = Some(parse_u64(&path, index, key, value)?),
                "max_relative_mad" => {
                    max_relative_mad = Some(parse_f64(&path, index, key, value)?)
                }
                _ => {
                    return Err(format!(
                        "{}:{}: unknown config key {key:?}",
                        path.display(),
                        index + 1
                    ));
                }
            }
        }

        let config = Self {
            name: required(name, "name", &path)?,
            warmup: required(warmup, "warmup", &path)?,
            samples: required(samples, "samples", &path)?,
            timeout: Duration::from_millis(required(timeout_ms, "timeout_ms", &path)?),
            max_relative_mad: required(max_relative_mad, "max_relative_mad", &path)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("benchmark name must not be empty".to_owned());
        }
        if self.samples < 3 {
            return Err("samples must be at least 3".to_owned());
        }
        if self.timeout.is_zero() {
            return Err("timeout_ms must be greater than zero".to_owned());
        }
        if !self.max_relative_mad.is_finite() || self.max_relative_mad < 0.0 {
            return Err("max_relative_mad must be a finite non-negative number".to_owned());
        }
        Ok(())
    }
}

pub(crate) fn safe_file_name(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn required<T>(value: Option<T>, key: &str, path: &Path) -> Result<T, String> {
    value.ok_or_else(|| format!("{}: missing required key {key:?}", path.display()))
}

fn parse_usize(path: &Path, index: usize, key: &str, value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
}

fn parse_u64(path: &Path, index: usize, key: &str, value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
}

fn parse_f64(path: &Path, index: usize, key: &str, value: &str) -> Result<f64, String> {
    value.parse::<f64>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
}

#[cfg(test)]
mod tests {
    use super::safe_file_name;

    #[test]
    fn sanitizes_case_name_for_output_path() {
        assert_eq!(safe_file_name("arithmetic / smoke"), "arithmetic___smoke");
    }
}
