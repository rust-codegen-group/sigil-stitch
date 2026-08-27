#!/usr/bin/env bash
set -euo pipefail

readonly EXPECTED_VERSION="cargo-semver-checks 0.49.0"
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly DEFAULT_ALLOWLIST="${REPO_ROOT}/tests/compatibility/semver-0.6.8-allowlist.txt"

die() {
  echo "semver-check: $*" >&2
  exit 1
}

parse_output() {
  local input_file="$1"
  local output_file="$2"

  awk '
    function close_failure() {
      if (failure_active && failure_records == 0) {
        print "malformed failure block for lint " lint > "/dev/stderr"
        malformed = 1
      }
    }

    /^[[:space:]]*Checking [^ ]+ v/ {
      package_name = $2
      next
    }

    /^--- failure [a-z0-9_]+:/ {
      close_failure()
      lint = $0
      sub(/^--- failure /, "", lint)
      sub(/:.*/, "", lint)
      failure_active = 1
      failure_records = 0
      reading_records = 0
      next
    }

    /^--- / {
      print "unrecognized semver record header: " $0 > "/dev/stderr"
      malformed = 1
      next
    }

    /^Failed in:$/ {
      if (!failure_active || package_name == "") {
        print "Failed in section without package or failure header" > "/dev/stderr"
        malformed = 1
      }
      reading_records = 1
      next
    }

    reading_records && /^  / {
      record = $0
      sub(/^  /, "", record)
      item = ""

      if (record ~ /^method [A-Za-z_][A-Za-z0-9_]* of trait [A-Za-z_][A-Za-z0-9_]*,/) {
        split(record, fields, /[ ,]+/)
        item = fields[5] "::" fields[2]
      } else if (record ~ /^enum [A-Za-z_][A-Za-z0-9_]* in /) {
        split(record, fields, /[ ]+/)
        item = fields[2]
      } else if (record ~ /^function [A-Za-z_][A-Za-z0-9_]* /) {
        split(record, fields, /[ ]+/)
        item = fields[2]
      } else if (record ~ /^[A-Za-z_][A-Za-z0-9_:]*, previously in file /) {
        item = record
        sub(/,.*/, "", item)
      } else if (record ~ /^[A-Za-z_][A-Za-z0-9_:]* now /) {
        split(record, fields, /[ ]+/)
        item = fields[1]
      } else {
        print "unrecognized Failed in record: " record > "/dev/stderr"
        malformed = 1
        next
      }

      print package_name "|" lint "|" item
      failure_records++
      next
    }

    reading_records && !/^  / {
      reading_records = 0
    }

    END {
      close_failure()
      if (malformed) {
        exit 42
      }
    }
  ' "${input_file}" > "${output_file}"
}

validate_completed_output() {
  local input_file="$1"

  awk '
    /^[[:space:]]*Checking [^ ]+ v/ {
      checks++
      next
    }

    /^[[:space:]]*Summary / {
      summaries++
      next
    }

    /^[[:space:]]*Finished \[[^]]+\] / {
      finished++
      finished_line = NR
    }

    NF { last_nonblank = NR }

    END {
      if (checks == 0) {
        print "semver output contains no package check" > "/dev/stderr"
        malformed = 1
      }
      if (summaries != checks) {
        print "semver output has " summaries " summaries for " checks " package checks" > "/dev/stderr"
        malformed = 1
      }
      if (finished != 1) {
        print "semver output has " finished " terminal Finished records" > "/dev/stderr"
        malformed = 1
      } else if (last_nonblank != finished_line) {
        print "semver output continues after its terminal Finished record" > "/dev/stderr"
        malformed = 1
      }
      if (malformed) {
        exit 42
      }
    }
  ' "${input_file}"
}

normalize_allowlist() {
  local allowlist_file="$1"
  local output_file="$2"

  awk '
    /^[[:space:]]*($|#)/ { next }
    /^[A-Za-z0-9_.-]+\|[a-z0-9_]+\|[A-Za-z_][A-Za-z0-9_:]*$/ { print; next }
    {
      print "malformed allowlist record at line " NR ": " $0 > "/dev/stderr"
      malformed = 1
    }
    END { if (malformed) exit 42 }
  ' "${allowlist_file}" > "${output_file}"
}

compare_output() (
  local semver_output="$1"
  local allowlist_file="$2"
  local comparison_dir
  comparison_dir="$(mktemp -d "${TMPDIR:-/tmp}/sigil-semver-compare.XXXXXX")"
  trap 'rm -rf "${comparison_dir}"' EXIT

  local parsed_raw="${comparison_dir}/parsed-raw.txt"
  local actual="${comparison_dir}/actual.txt"
  local approved_raw="${comparison_dir}/approved-raw.txt"
  local approved="${comparison_dir}/approved.txt"
  local duplicates="${comparison_dir}/duplicates.txt"
  local missing="${comparison_dir}/missing.txt"
  local unexpected="${comparison_dir}/unexpected.txt"

  validate_completed_output "${semver_output}" || die \
    "cargo-semver-checks 0.49.0 output was malformed"
  parse_output "${semver_output}" "${parsed_raw}" || die \
    "cargo-semver-checks 0.49.0 output was malformed"
  LC_ALL=C sort -u "${parsed_raw}" > "${actual}"

  normalize_allowlist "${allowlist_file}" "${approved_raw}" || die "semver allowlist was malformed"
  LC_ALL=C sort "${approved_raw}" | uniq -d > "${duplicates}"
  if [[ -s "${duplicates}" ]]; then
    sed 's/^/duplicate approved record: /' "${duplicates}" >&2
    die "semver allowlist contains duplicate records"
  fi
  LC_ALL=C sort "${approved_raw}" > "${approved}"

  comm -23 "${approved}" "${actual}" > "${missing}"
  comm -13 "${approved}" "${actual}" > "${unexpected}"

  if [[ -s "${missing}" ]]; then
    sed 's/^/missing approved record: /' "${missing}" >&2
  fi
  if [[ -s "${unexpected}" ]]; then
    sed 's/^/unexpected semver record: /' "${unexpected}" >&2
  fi
  if [[ -s "${missing}" || -s "${unexpected}" ]]; then
    die "semver output does not exactly match the approved set"
  fi

  local record_count
  record_count="$(wc -l < "${actual}" | tr -d ' ')"
  echo "semver-check: ${record_count} approved compatibility break(s)"
)

if [[ "${1:-}" == "--compare" ]]; then
  [[ "$#" -eq 3 ]] || die "usage: $0 --compare <semver-output> <allowlist>"
  compare_output "$2" "$3"
  exit 0
fi

[[ "$#" -eq 0 ]] || die "usage: $0 [--compare <semver-output> <allowlist>]"

actual_version="$(cargo semver-checks --version 2>/dev/null || true)"
[[ "${actual_version}" == "${EXPECTED_VERSION}" ]] || die \
  "expected ${EXPECTED_VERSION}; found ${actual_version:-not installed}"

run_dir="$(mktemp -d "${TMPDIR:-/tmp}/sigil-semver-run.XXXXXX")"
trap 'rm -rf "${run_dir}"' EXIT
semver_output="${run_dir}/cargo-semver-checks.txt"

set +e
(
  cd "${REPO_ROOT}"
  cargo semver-checks check-release \
    --workspace \
    --baseline-rev 0.6.8 \
    --color never
) > "${semver_output}" 2>&1
semver_exit=$?
set -e

if [[ "${semver_exit}" -ne 0 ]] && ! grep -q '^--- failure ' "${semver_output}"; then
  cat "${semver_output}" >&2
  die "cargo-semver-checks failed before producing a compatibility report"
fi

compare_output "${semver_output}" "${DEFAULT_ALLOWLIST}"
cat "${semver_output}"
