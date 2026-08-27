use std::path::{Path, PathBuf};

/// Assert that generated output matches its checked fixture.
///
/// The worktree is changed only when `BLESS=1` is set explicitly. A missing or
/// mismatched fixture otherwise fails closed.
pub fn assert_golden(name: &str, actual: &str) {
    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-goldens");
    assert_golden_at(&golden_dir, name, actual, std::env::var("BLESS").is_ok());
}

fn assert_golden_at(golden_dir: &Path, name: &str, actual: &str, bless: bool) {
    let golden_path = golden_dir.join(name);

    if bless {
        if let Some(parent) = golden_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&golden_path, actual).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&golden_path).unwrap_or_else(|error| {
        panic!(
            "missing golden file {}: {error}; run with BLESS=1 to create it",
            golden_path.display()
        )
    });
    if actual != expected {
        eprintln!("Golden file mismatch: {}", golden_path.display());
        eprintln!("--- expected ---");
        eprintln!("{expected}");
        eprintln!("--- actual ---");
        eprintln!("{actual}");
        eprintln!("---");
        eprintln!("Run with BLESS=1 to update golden files.");
        panic!("Golden file mismatch: {}", golden_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn isolated_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sigil-stitch-golden-helper-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn missing_fixture_does_not_write_without_blessing() {
        let dir = isolated_dir();
        let result = std::panic::catch_unwind(|| {
            assert_golden_at(&dir, "missing/output.txt", "actual", false);
        });

        assert!(result.is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn mismatched_fixture_does_not_change_without_blessing() {
        let dir = isolated_dir();
        assert_golden_at(&dir, "output.txt", "expected", true);

        let result = std::panic::catch_unwind(|| {
            assert_golden_at(&dir, "output.txt", "actual", false);
        });

        assert!(result.is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("output.txt")).unwrap(),
            "expected"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn matching_fixture_succeeds_without_blessing() {
        let dir = isolated_dir();
        assert_golden_at(&dir, "output.txt", "expected", true);

        assert_golden_at(&dir, "output.txt", "expected", false);

        assert_eq!(
            std::fs::read_to_string(dir.join("output.txt")).unwrap(),
            "expected"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn blessing_is_the_only_write_path() {
        let dir = isolated_dir();
        assert_golden_at(&dir, "nested/output.txt", "first", true);
        assert_golden_at(&dir, "nested/output.txt", "second", true);

        assert_eq!(
            std::fs::read_to_string(dir.join("nested/output.txt")).unwrap(),
            "second"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
