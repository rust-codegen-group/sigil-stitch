//! Local compiler acceptance for freshly generated positive/negative fixtures.

use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Deserialize)]
struct Fixture {
    compiler: String,
    #[serde(default)]
    command_prefix: Vec<String>,
    version_flags: Vec<String>,
    flags: Vec<String>,
    positive: String,
    negative: Vec<Negative>,
}

#[derive(Deserialize)]
struct Negative {
    file: String,
    diagnostic: String,
}

// Own only the newly created directory, never a pre-existing path.
struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> std::io::Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sigil-stitch-source-{}-{timestamp}",
            std::process::id()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("could not clean {}: {error}", self.0.display());
        }
    }
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run(fixture: &Fixture, arguments: &[String], directory: &Path) -> std::io::Result<Output> {
    Command::new(&fixture.compiler)
        .args(&fixture.command_prefix)
        .args(arguments)
        .current_dir(directory)
        .output()
}

fn intended_failure(
    filename: &str,
    source: &str,
    output: &str,
    diagnostic: &str,
) -> Result<bool, regex::Error> {
    let markers: Vec<_> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("acceptance-failure"))
        .map(|(index, _)| index + 1)
        .collect();
    let [line] = markers.as_slice() else {
        return Ok(false);
    };
    let location = Regex::new(&format!(
        r"{}(?::{line}:\d+|\({line},\d+\))",
        regex::escape(filename)
    ))?;
    let error = Regex::new(r"(?i)\berror\b")?;
    let any_location = Regex::new(r":\d+:\d+|\(\d+,\d+\)")?;
    let expected = Regex::new(&format!("(?s:{diagnostic})"))?;
    let lines: Vec<_> = output.lines().collect();
    for (index, header) in lines.iter().enumerate() {
        if !location.is_match(header) || !error.is_match(header) {
            continue;
        }
        // An unrelated later error must not supply the expected diagnosis.
        let end = lines[index + 1..]
            .iter()
            .position(|line| error.is_match(line) && any_location.is_match(line))
            .map_or(lines.len(), |offset| index + 1 + offset);
        // Locations and fixture names are evidence of the use-site, not diagnosis.
        let diagnosis = lines[index..end].join("\n").replace(filename, "");
        if expected.is_match(&diagnosis) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn selected_languages(
    arguments: impl IntoIterator<Item = String>,
    manifest: &BTreeMap<String, Fixture>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut arguments = arguments.into_iter();
    let mut languages = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument != "--language" {
            return Err(format!("unexpected argument: {argument}; use --language <name>").into());
        }
        let language = arguments.next().ok_or("--language requires a name")?;
        if !manifest.contains_key(&language) {
            return Err(format!("unknown language: {language}").into());
        }
        languages.push(language);
    }
    if languages.is_empty() {
        languages.extend(manifest.keys().cloned());
    }
    Ok(languages)
}

fn main() -> Result<(), Box<dyn Error>> {
    let manifest: BTreeMap<String, Fixture> =
        serde_json::from_str(include_str!("../tests/generated-source/manifest.json"))?;
    let languages = selected_languages(std::env::args().skip(1), &manifest)?;
    let temporary = TemporaryDirectory::new()?;
    let directory = &temporary.0;
    let generation = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--example",
            "generated_source_acceptance",
            "--",
        ])
        .arg(directory)
        .current_dir(ROOT)
        .output()?;
    if !generation.status.success() {
        return Err(format!("source generation failed:\n{}", output_text(&generation)).into());
    }
    let scala_version = Regex::new(r"(?i)(?:version\s+|compiler\s+)3\.")?;
    for language in languages {
        let fixture = &manifest[&language];
        let version = run(fixture, &fixture.version_flags, directory).map_err(|error| {
            format!("cannot run required compiler {}: {error}", fixture.compiler)
        })?;
        let version_text = output_text(&version);
        if !version.status.success() {
            return Err(format!("compiler version query failed:\n{version_text}").into());
        }
        println!("{language}: {}", version_text.trim());
        let mut flags = fixture.flags.clone();
        if language == "scala" {
            if !scala_version.is_match(&version_text) {
                return Err("Scala acceptance requires a Scala 3 compiler".into());
            }
            let classes = directory.join("scala-classes");
            std::fs::create_dir_all(&classes)?;
            flags.extend(["-d".into(), classes.to_string_lossy().into_owned()]);
        }
        let mut arguments = flags.clone();
        arguments.push(fixture.positive.clone());
        let positive = run(fixture, &arguments, directory)?;
        if !positive.status.success() {
            return Err(format!("positive fixture failed:\n{}", output_text(&positive)).into());
        }
        for negative in &fixture.negative {
            let source = std::fs::read_to_string(directory.join(&negative.file))?;
            let mut arguments = flags.clone();
            arguments.push(negative.file.clone());
            let result = run(fixture, &arguments, directory)?;
            if result.status.success()
                || !intended_failure(
                    &negative.file,
                    &source,
                    &output_text(&result),
                    &negative.diagnostic,
                )?
            {
                return Err(format!(
                    "negative fixture did not fail at its intended use-site:\n{}",
                    output_text(&result)
                )
                .into());
            }
            println!("  verified {}", negative.file);
        }
        println!("  positive and intended negatives passed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(output: &str) -> bool {
        intended_failure(
            "negative.ts",
            "declare const f: (x: string) => void;\nf(1); // acceptance-failure\n",
            output,
            "TS2345",
        )
        .unwrap()
    }

    #[test]
    fn typescript_location_and_diagnostic() {
        assert!(check("negative.ts(2,3): error TS2345: wrong argument"));
    }

    #[test]
    fn colon_location_and_diagnostic() {
        assert!(check("negative.ts:2:3: error: TS2345 wrong argument"));
    }

    #[test]
    fn unrelated_setup_error_does_not_pass() {
        assert!(!check("negative.ts(1,1): error TS2307: missing module"));
        assert!(!check("negative.ts(2,3): error TS2307: missing module"));
    }

    #[test]
    fn note_at_use_site_does_not_pass() {
        assert!(!check(
            "negative.ts:1:1: error: TS2345\nnegative.ts:2:3: note: instantiated here"
        ));
    }

    #[test]
    fn another_error_cannot_supply_expected_diagnostic() {
        assert!(!check(
            "negative.ts:2:3: error: unrelated\nnegative.ts:1:1: error: TS2345"
        ));
    }

    #[test]
    fn missing_or_duplicate_marker_does_not_pass() {
        for source in [
            "f(1);",
            "// acceptance-failure\nf(1); // acceptance-failure",
        ] {
            assert!(
                !intended_failure(
                    "negative.ts",
                    source,
                    "negative.ts(2,3): error TS2345",
                    "TS2345"
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn scala_filename_cannot_supply_kind_diagnosis() {
        let source = "val invalid = unknownName // acceptance-failure";
        let unrelated = "-- [E006] Not Found Error: scala_wrong_kind.scala:1:15\n1 |val invalid = unknownName\n  |Not found: unknownName";
        assert!(
            !intended_failure("scala_wrong_kind.scala", source, unrelated, "(?i:kind)").unwrap()
        );
        assert!(
            !intended_failure(
                "scala_wrong_kind.scala",
                source,
                unrelated,
                "Type argument Int does not conform to upper bound \\[_\\] =>> Any"
            )
            .unwrap()
        );
        let actual = "-- [E057] Type Mismatch Error: scala_wrong_kind.scala:1:15\n  |Type argument Int does not conform to upper bound [_] =>> Any";
        assert!(
            intended_failure(
                "scala_wrong_kind.scala",
                source,
                actual,
                "Type argument Int does not conform to upper bound \\[_\\] =>> Any"
            )
            .unwrap()
        );
    }
}
