use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CACHE_LAYOUT: &str = "run-cache-v0";
const COMPLETE_MARKER: &[u8] = b"complete-v0\n";
const MAX_CACHE_ENTRIES: usize = 32;
const STALE_STAGING_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug)]
pub(crate) struct RunCache {
    root: PathBuf,
    key: String,
    evolution_source: Vec<u8>,
    generated_source: Vec<u8>,
    compiler_fingerprint: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct StagingEntry {
    pub(crate) dir: PathBuf,
    pub(crate) binary: PathBuf,
    final_dir: PathBuf,
}

#[derive(Debug)]
pub(crate) struct RunnableBinary {
    path: PathBuf,
    cleanup_dir: Option<PathBuf>,
}

impl RunnableBinary {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn cleanup(&self) {
        if let Some(dir) = &self.cleanup_dir {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

impl StagingEntry {
    pub(crate) fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl RunCache {
    pub(crate) fn new(
        evolution_source: &str,
        generated_source: &str,
        compiler_fingerprint: &str,
    ) -> Result<Self, String> {
        let root = cache_root().join(CACHE_LAYOUT);
        fs::create_dir_all(&root)
            .map_err(|error| format!("failed to create run cache {}: {error}", root.display()))?;

        cleanup_stale_staging(&root);

        let key = stable_cache_key(&[
            evolution_source.as_bytes(),
            generated_source.as_bytes(),
            compiler_fingerprint.as_bytes(),
        ]);

        Ok(Self {
            root,
            key,
            evolution_source: evolution_source.as_bytes().to_vec(),
            generated_source: generated_source.as_bytes().to_vec(),
            compiler_fingerprint: compiler_fingerprint.as_bytes().to_vec(),
        })
    }

    pub(crate) fn lookup(&self) -> Option<RunnableBinary> {
        let prefix = format!("entry-{}-", self.key);
        let entries = fs::read_dir(&self.root).ok()?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with(&prefix) {
                continue;
            }

            let dir = entry.path();
            if self.entry_is_valid(&dir) {
                return Some(RunnableBinary {
                    path: dir.join(binary_name()),
                    cleanup_dir: None,
                });
            }
        }

        None
    }

    pub(crate) fn prepare_staging(&self) -> Result<StagingEntry, String> {
        let generation = unique_generation()?;
        let staging_dir = self
            .root
            .join(format!("staging-{}-{generation}", self.key));
        let final_dir = self.root.join(format!("entry-{}-{generation}", self.key));

        fs::create_dir(&staging_dir).map_err(|error| {
            format!(
                "failed to create run-cache staging directory {}: {error}",
                staging_dir.display()
            )
        })?;

        if let Err(error) = self.write_identity_files(&staging_dir) {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }

        Ok(StagingEntry {
            binary: staging_dir.join(binary_name()),
            dir: staging_dir,
            final_dir,
        })
    }

    pub(crate) fn publish(&self, staging: StagingEntry) -> RunnableBinary {
        let binary = staging.binary.clone();

        if fs::write(staging.dir.join("complete"), COMPLETE_MARKER).is_err() {
            return RunnableBinary {
                path: binary,
                cleanup_dir: Some(staging.dir),
            };
        }

        match fs::rename(&staging.dir, &staging.final_dir) {
            Ok(()) => {
                prune_cache_entries(&self.root, &staging.final_dir);
                RunnableBinary {
                    path: staging.final_dir.join(binary_name()),
                    cleanup_dir: None,
                }
            }
            Err(_) => RunnableBinary {
                path: binary,
                cleanup_dir: Some(staging.dir),
            },
        }
    }

    fn write_identity_files(&self, dir: &Path) -> Result<(), String> {
        fs::write(dir.join("source.evo"), &self.evolution_source)
            .map_err(|error| format!("failed to write cached Evolution source: {error}"))?;
        fs::write(dir.join("generated.rs"), &self.generated_source)
            .map_err(|error| format!("failed to write cached generated Rust: {error}"))?;
        fs::write(dir.join("compiler.txt"), &self.compiler_fingerprint)
            .map_err(|error| format!("failed to write cached compiler fingerprint: {error}"))?;
        Ok(())
    }

    fn entry_is_valid(&self, dir: &Path) -> bool {
        if fs::read(dir.join("complete")).ok().as_deref() != Some(COMPLETE_MARKER) {
            return false;
        }
        if fs::read(dir.join("source.evo")).ok().as_deref() != Some(&self.evolution_source) {
            return false;
        }
        if fs::read(dir.join("generated.rs")).ok().as_deref() != Some(&self.generated_source) {
            return false;
        }
        if fs::read(dir.join("compiler.txt")).ok().as_deref()
            != Some(&self.compiler_fingerprint)
        {
            return false;
        }

        fs::metadata(dir.join(binary_name()))
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
    }
}

fn cache_root() -> PathBuf {
    if let Some(path) = env::var_os("EVO_CACHE_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    #[cfg(windows)]
    if let Some(path) = env::var_os("LOCALAPPDATA").filter(|value| !value.is_empty()) {
        return PathBuf::from(path).join("RustEvolution");
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join("Library/Caches/rust-evolution");
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if let Some(path) = env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
            return PathBuf::from(path).join("rust-evolution");
        }
        if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
            return PathBuf::from(home).join(".cache/rust-evolution");
        }
    }

    env::temp_dir().join("rust-evolution-cache")
}

fn binary_name() -> OsString {
    OsString::from(format!("program{}", env::consts::EXE_SUFFIX))
}

fn unique_generation() -> Result<String, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    Ok(format!("{}-{nanos}", process::id()))
}

fn cleanup_stale_staging(root: &Path) {
    let now = SystemTime::now();
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("staging-") {
            continue;
        }
        let path = entry.path();
        let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age >= STALE_STAGING_AGE {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn prune_cache_entries(root: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    let mut cached = entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("entry-"))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();

    if cached.len() <= MAX_CACHE_ENTRIES {
        return;
    }

    cached.sort_by_key(|(modified, _)| *modified);
    let remove_count = cached.len() - MAX_CACHE_ENTRIES;
    for (_, path) in cached.into_iter().filter(|(_, path)| path != keep).take(remove_count) {
        let _ = fs::remove_dir_all(path);
    }
}

fn stable_cache_key(parts: &[&[u8]]) -> String {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    const OFFSETS: [u64; 4] = [
        0xcbf2_9ce4_8422_2325,
        0x8422_2325_cbf2_9ce4,
        0x9e37_79b9_7f4a_7c15,
        0xd6e8_feb8_6659_fd93,
    ];

    let mut hashes = OFFSETS;
    for part in parts {
        for hash in &mut hashes {
            *hash ^= part.len() as u64;
            *hash = hash.wrapping_mul(PRIME);
        }
        for byte in *part {
            for (index, hash) in hashes.iter_mut().enumerate() {
                *hash ^= u64::from(*byte).wrapping_add((index as u64) << 8);
                *hash = hash.wrapping_mul(PRIME);
                *hash ^= *hash >> (13 + index);
            }
        }
    }

    format!(
        "{:016x}{:016x}{:016x}{:016x}",
        hashes[0], hashes[1], hashes[2], hashes[3]
    )
}

#[cfg(test)]
mod tests {
    use super::{COMPLETE_MARKER, RunCache, stable_cache_key};
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("evo-run-cache-{label}-{}-{nanos}", process::id()))
    }

    fn cache_for(root: &std::path::Path) -> RunCache {
        RunCache {
            root: root.to_path_buf(),
            key: stable_cache_key(&[b"source", b"generated", b"compiler"]),
            evolution_source: b"source".to_vec(),
            generated_source: b"generated".to_vec(),
            compiler_fingerprint: b"compiler".to_vec(),
        }
    }

    #[test]
    fn cache_key_changes_for_each_compilation_identity_part() {
        let baseline = stable_cache_key(&[b"source", b"generated", b"compiler"]);
        assert_ne!(baseline, stable_cache_key(&[b"changed", b"generated", b"compiler"]));
        assert_ne!(baseline, stable_cache_key(&[b"source", b"changed", b"compiler"]));
        assert_ne!(baseline, stable_cache_key(&[b"source", b"generated", b"changed"]));
    }

    #[test]
    fn lookup_requires_exact_identity_files_and_completion_marker() {
        let root = temp_dir("exact");
        fs::create_dir_all(&root).expect("cache root should be created");
        let cache = cache_for(&root);
        let entry = root.join(format!("entry-{}-test", cache.key));
        fs::create_dir_all(&entry).expect("entry should be created");
        fs::write(entry.join("source.evo"), b"source").expect("source should be written");
        fs::write(entry.join("generated.rs"), b"generated").expect("generated should be written");
        fs::write(entry.join("compiler.txt"), b"compiler").expect("compiler should be written");
        fs::write(entry.join("program"), b"binary").expect("binary should be written");

        assert!(cache.lookup().is_none(), "incomplete entry must miss");
        fs::write(entry.join("complete"), COMPLETE_MARKER).expect("marker should be written");

        #[cfg(windows)]
        fs::rename(entry.join("program"), entry.join("program.exe"))
            .expect("windows binary should be renamed");

        assert!(cache.lookup().is_some(), "complete exact entry should hit");
        fs::write(entry.join("generated.rs"), b"corrupt").expect("generated should be corrupted");
        assert!(cache.lookup().is_none(), "corrupted metadata must miss");

        let _ = fs::remove_dir_all(root);
    }
}