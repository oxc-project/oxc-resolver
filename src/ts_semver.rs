//! Port of TypeScript's `src/compiler/semver.ts`, used to match `typesVersions` keys.
//!
//! TypeScript implements its own subset of node-semver, so the range grammar and the
//! prerelease handling here follow TypeScript rather than npm or Cargo semantics.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
}

impl Version {
    /// A version greater than any version a range can name.
    pub const LATEST: Self = Self { major: u64::MAX, minor: 0, patch: 0, prerelease: Vec::new() };

    const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self { major, minor, patch, prerelease: Vec::new() }
    }

    /// TS: `Version.tryParse`. Minor and patch may be omitted (`5`, `5.4`).
    pub fn parse(text: &str) -> Option<Self> {
        let (text, build) = split_build(text);
        if build.is_some_and(|build| !is_valid_build(build)) {
            return None;
        }
        let (core, prerelease) = split_prerelease(text);
        let mut parts = core.split('.');
        let major = parse_numeric(parts.next()?)?;
        let minor = parts.next();
        let patch = parts.next();
        // Like TS's regex, prerelease and build are only allowed after a full `x.y.z`.
        if parts.next().is_some() || ((prerelease.is_some() || build.is_some()) && patch.is_none())
        {
            return None;
        }
        let minor = minor.map_or(Some(0), parse_numeric)?;
        let patch = patch.map_or(Some(0), parse_numeric)?;
        let prerelease = parse_prerelease(prerelease)?;
        Some(Self { major, minor, patch, prerelease })
    }

    fn increment(&self, field: Field) -> Self {
        match field {
            Field::Major => Self::new(self.major.saturating_add(1), 0, 0),
            Field::Minor => Self::new(self.major, self.minor.saturating_add(1), 0),
            Field::Patch => Self::new(self.major, self.minor, self.patch.saturating_add(1)),
        }
    }

    fn with_prerelease_zero(mut self) -> Self {
        self.prerelease = vec!["0".to_string()];
        self
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| compare_prerelease(&self.prerelease, &other.prerelease))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// TS: `comparePrereleaseIdentifiers`
fn compare_prerelease(left: &[String], right: &[String]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => {}
    }
    for (l, r) in left.iter().zip(right) {
        if l == r {
            continue;
        }
        let ordering = match (is_numeric_identifier(l), is_numeric_identifier(r)) {
            (true, true) => {
                // Numeric identifiers may exceed u64; compare by length, then lexically.
                l.len().cmp(&r.len()).then_with(|| l.cmp(r))
            }
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => l.cmp(r),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

#[derive(Debug, Clone, Copy)]
enum Field {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

#[derive(Debug)]
struct Comparator {
    operator: Operator,
    operand: Version,
}

impl Comparator {
    fn test(&self, version: &Version) -> bool {
        let ordering = version.cmp(&self.operand);
        match self.operator {
            Operator::Lt => ordering.is_lt(),
            Operator::Le => ordering.is_le(),
            Operator::Gt => ordering.is_gt(),
            Operator::Ge => ordering.is_ge(),
            Operator::Eq => ordering.is_eq(),
        }
    }
}

/// TS: `VersionRange`
#[derive(Debug)]
pub struct VersionRange {
    alternatives: Vec<Vec<Comparator>>,
}

impl VersionRange {
    /// TS: `VersionRange.tryParse` / `parseRange`
    pub fn parse(text: &str) -> Option<Self> {
        let mut alternatives = vec![];
        for range in text.trim().split("||") {
            if range.is_empty() {
                continue;
            }
            let range = range.trim();
            let mut comparators = vec![];
            if let Some((left, right)) = match_hyphen(range) {
                parse_hyphen(left, right, &mut comparators)?;
            } else {
                // `"".split(/\s+/)` yields `[""]`, which fails to parse below.
                if range.is_empty() {
                    return None;
                }
                for simple in range.split_whitespace() {
                    let (operator, partial) = match_simple(simple)?;
                    parse_comparator(operator, partial, &mut comparators)?;
                }
            }
            alternatives.push(comparators);
        }
        Some(Self { alternatives })
    }

    pub fn test(&self, version: &Version) -> bool {
        self.alternatives.is_empty()
            || self
                .alternatives
                .iter()
                .any(|comparators| comparators.iter().all(|c| c.test(version)))
    }
}

/// TS: `hyphenRegExp`
fn match_hyphen(range: &str) -> Option<(&str, &str)> {
    let mut parts = range.split_whitespace();
    let (left, hyphen, right) = (parts.next()?, parts.next()?, parts.next()?);
    (parts.next().is_none() && hyphen == "-" && is_range_operand(left) && is_range_operand(right))
        .then_some((left, right))
}

/// TS: `rangeRegExp`
fn match_simple(simple: &str) -> Option<(&str, &str)> {
    let operator = ["<=", ">=", "~", "^", "<", ">", "="]
        .into_iter()
        .find(|operator| simple.starts_with(operator))
        .unwrap_or("");
    let partial = &simple[operator.len()..];
    is_range_operand(partial).then_some((operator, partial))
}

/// TS: `[a-z0-9-+.*]+` with the `i` flag
fn is_range_operand(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.' | b'*'))
}

struct Partial<'a> {
    version: Version,
    major: &'a str,
    minor: &'a str,
    patch: &'a str,
}

/// TS: `parsePartial`. Omitted minor and patch are treated as wildcards.
fn parse_partial(text: &str) -> Option<Partial<'_>> {
    let (text, build) = split_build(text);
    if build.is_some_and(|build| !is_valid_build(build)) {
        return None;
    }
    let (core, prerelease) = split_prerelease(text);
    let mut parts = core.split('.');
    let major = parts.next()?;
    let minor = parts.next();
    let patch = parts.next();
    if parts.next().is_some() || ((prerelease.is_some() || build.is_some()) && patch.is_none()) {
        return None;
    }
    let (minor, patch) = (minor.unwrap_or("*"), patch.unwrap_or("*"));
    let component = |part: &str| if is_wildcard(part) { Some(0) } else { parse_numeric(part) };
    let (major_num, minor_num, patch_num) =
        (component(major)?, component(minor)?, component(patch)?);
    let version = Version {
        major: major_num,
        minor: if is_wildcard(major) { 0 } else { minor_num },
        patch: if is_wildcard(major) || is_wildcard(minor) { 0 } else { patch_num },
        prerelease: parse_prerelease(prerelease)?,
    };
    Some(Partial { version, major, minor, patch })
}

/// TS: `parseHyphen`
fn parse_hyphen(left: &str, right: &str, comparators: &mut Vec<Comparator>) -> Option<()> {
    let left = parse_partial(left)?;
    let right = parse_partial(right)?;
    if !is_wildcard(left.major) {
        comparators.push(Comparator { operator: Operator::Ge, operand: left.version });
    }
    if !is_wildcard(right.major) {
        comparators.push(if is_wildcard(right.minor) {
            Comparator { operator: Operator::Lt, operand: right.version.increment(Field::Major) }
        } else if is_wildcard(right.patch) {
            Comparator { operator: Operator::Lt, operand: right.version.increment(Field::Minor) }
        } else {
            Comparator { operator: Operator::Le, operand: right.version }
        });
    }
    Some(())
}

/// TS: `parseComparator`
fn parse_comparator(operator: &str, text: &str, comparators: &mut Vec<Comparator>) -> Option<()> {
    let Partial { version, major, minor, patch } = parse_partial(text)?;
    let mut push = |operator, operand| comparators.push(Comparator { operator, operand });
    if is_wildcard(major) {
        if matches!(operator, "<" | ">") {
            push(Operator::Lt, Version::new(0, 0, 0).with_prerelease_zero());
        }
        return Some(());
    }
    match operator {
        "~" => {
            let upper =
                version.increment(if is_wildcard(minor) { Field::Major } else { Field::Minor });
            push(Operator::Ge, version);
            push(Operator::Lt, upper);
        }
        "^" => {
            let field = if version.major > 0 || is_wildcard(minor) {
                Field::Major
            } else if version.minor > 0 || is_wildcard(patch) {
                Field::Minor
            } else {
                Field::Patch
            };
            let upper = version.increment(field);
            push(Operator::Ge, version);
            push(Operator::Lt, upper);
        }
        "<" | ">=" => {
            let operator = if operator == "<" { Operator::Lt } else { Operator::Ge };
            if is_wildcard(minor) || is_wildcard(patch) {
                push(operator, version.with_prerelease_zero());
            } else {
                push(operator, version);
            }
        }
        "<=" | ">" => {
            let wildcard_operator = if operator == "<=" { Operator::Lt } else { Operator::Ge };
            if is_wildcard(minor) {
                push(wildcard_operator, version.increment(Field::Major).with_prerelease_zero());
            } else if is_wildcard(patch) {
                push(wildcard_operator, version.increment(Field::Minor).with_prerelease_zero());
            } else {
                push(if operator == "<=" { Operator::Le } else { Operator::Gt }, version);
            }
        }
        "=" | "" => {
            if is_wildcard(minor) || is_wildcard(patch) {
                let field = if is_wildcard(minor) { Field::Major } else { Field::Minor };
                let upper = version.increment(field).with_prerelease_zero();
                push(Operator::Ge, version.with_prerelease_zero());
                push(Operator::Lt, upper);
            } else {
                push(Operator::Eq, version);
            }
        }
        _ => return None,
    }
    Some(())
}

fn is_wildcard(part: &str) -> bool {
    matches!(part, "*" | "x" | "X")
}

fn split_build(text: &str) -> (&str, Option<&str>) {
    text.split_once('+').map_or((text, None), |(text, build)| (text, Some(build)))
}

fn split_prerelease(text: &str) -> (&str, Option<&str>) {
    text.split_once('-').map_or((text, None), |(core, prerelease)| (core, Some(prerelease)))
}

/// `0|[1-9]\d*`
fn is_numeric_identifier(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
}

fn parse_numeric(s: &str) -> Option<u64> {
    if is_numeric_identifier(s) { s.parse().ok() } else { None }
}

/// TS: `prereleaseRegExp`. Each identifier is numeric without leading zeros, or alphanumeric.
fn parse_prerelease(prerelease: Option<&str>) -> Option<Vec<String>> {
    let Some(prerelease) = prerelease else { return Some(vec![]) };
    prerelease
        .split('.')
        .map(|part| {
            let valid = is_numeric_identifier(part)
                || (part.bytes().next().is_some_and(|b| b.is_ascii_alphabetic() || b == b'-')
                    && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
            valid.then(|| part.to_string())
        })
        .collect()
}

/// TS: `buildRegExp`
fn is_valid_build(build: &str) -> bool {
    build.split('.').all(|part| {
        !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

#[cfg(test)]
mod test {
    use super::{Version, VersionRange};

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn assert_range(range: &str, version: &str, expected: bool) {
        let parsed = VersionRange::parse(range).unwrap_or_else(|| panic!("invalid range {range}"));
        assert_eq!(parsed.test(&v(version)), expected, "{range} test {version}");
    }

    #[test]
    fn parse_version() {
        assert_eq!(v("1"), v("1.0.0"));
        assert_eq!(v("1.2"), v("1.2.0"));
        assert_eq!(v("1.2.3-pre.4+build.5").prerelease, ["pre", "4"]);
        assert_eq!(v("1.2.3+build"), v("1.2.3"));
        for invalid in ["", "x", "1.2.3.4", "01.2.3", "1.2-pre", "1.2.3-01", "1.2.3-", "1.2.3+"] {
            assert!(Version::parse(invalid).is_none(), "{invalid}");
        }
    }

    // Ported from TypeScript's `src/testRunner/unittests/semver.ts`
    #[test]
    fn compare_version() {
        assert_eq!(v("1.0.0"), v("1.0.0"));
        assert!(v("1.0.0") > v("0.0.0"));
        assert!(v("1.0.0") > v("0.1.0"));
        assert!(v("1.0.0") > v("0.0.1"));
        assert!(v("1.1.0") > v("1.0.0"));
        assert!(v("1.0.1") > v("1.0.0"));
        assert!(v("1.0.0") > v("1.0.0-pre"));
        assert!(v("1.0.0-pre.1") > v("1.0.0-pre"));
        assert!(v("1.0.0-pre.1") < v("1.0.0-pre.2"));
        assert!(v("1.0.0-pre.10") > v("1.0.0-pre.2"));
        assert!(v("1.0.0-pre.a") > v("1.0.0-pre.1"));
        assert!(v("1.0.0-a") < v("1.0.0-b"));
        assert_eq!(v("1.0.0+build"), v("1.0.0"));
        assert!(Version::LATEST > v("99999.0.0"));
    }

    #[test]
    fn range_parse() {
        for valid in [
            "",
            "*",
            "x",
            "1",
            "1.2",
            "1.2.3",
            "<=5.6",
            ">=3.1.0-0",
            "1 - 2",
            "~1 || ^2",
            "||",
            "1 || ",
        ] {
            assert!(VersionRange::parse(valid).is_some(), "{valid}");
        }
        for invalid in ["< 1", "1 || || 2", "1.2.3.4", "01", "a", "!1", "1.2-pre", "1 - 2 - 3"] {
            assert!(VersionRange::parse(invalid).is_none(), "{invalid}");
        }
    }

    #[test]
    fn range_wildcard() {
        for range in ["", "*", "x", "X", "* || 1", "<=* ", ">=*"] {
            assert_range(range, "1.0.0", true);
            assert_range(range, "0.0.0-0", true);
        }
        assert_range("<*", "0.0.0", false);
        assert_range(">*", "1.0.0", false);
    }

    #[test]
    fn range_partial() {
        assert_range("1", "1.0.0", true);
        assert_range("1", "1.9.9", true);
        assert_range("1", "2.0.0-0", false);
        assert_range("1", "1.0.0-0", true);
        assert_range("1", "0.9.9", false);
        assert_range("1.2", "1.2.0", true);
        assert_range("1.2", "1.2.9", true);
        assert_range("1.2", "1.3.0", false);
        assert_range("1.2.x", "1.2.5", true);
        assert_range("1.2.3", "1.2.3", true);
        assert_range("1.2.3", "1.2.4", false);
        assert_range("=1.2.3", "1.2.3", true);
    }

    #[test]
    fn range_hyphen() {
        assert_range("1.0.0 - 2.0.0", "1.0.0", true);
        assert_range("1.0.0 - 2.0.0", "2.0.0", true);
        assert_range("1.0.0 - 2.0.0", "2.0.1", false);
        assert_range("1.0.0 - 2", "2.9.9", true);
        assert_range("1.0.0 - 2", "3.0.0", false);
        assert_range("1.0.0 - 2.1", "2.1.9", true);
        assert_range("1.0.0 - 2.1", "2.2.0", false);
        assert_range("* - 2", "0.0.0", true);
        assert_range("1 - *", "9.9.9", true);
        assert_range("1 - *", "0.9.9", false);
    }

    #[test]
    fn range_comparators() {
        assert_range(">1.0.0", "1.0.1", true);
        assert_range(">1.0.0", "1.0.0", false);
        assert_range(">1", "2.0.0", true);
        assert_range(">1", "1.9.9", false);
        assert_range(">1.2", "1.3.0", true);
        assert_range(">1.2", "1.2.9", false);
        assert_range(">=1.0.0", "1.0.0", true);
        assert_range(">=1.0.0", "1.0.0-0", false);
        assert_range(">=1", "1.0.0-0", true);
        assert_range("<2.0.0", "1.9.9", true);
        assert_range("<2.0.0", "2.0.0-0", true);
        assert_range("<2", "2.0.0-0", false);
        assert_range("<=2.0.0", "2.0.0", true);
        assert_range("<=2.0.0", "2.0.1", false);
        assert_range("<=5.6", "5.6.3", true);
        assert_range("<=5.6", "5.7.0-dev", false);
        assert_range("<=5.6", "6.0.3", false);
        assert_range(">=3.1.0-0", "3.1.0-dev.1", true);
        assert_range(">=1.0.0 <2.0.0", "1.5.0", true);
        assert_range(">=1.0.0 <2.0.0", "2.0.0", false);
        assert_range("<1 || >=2", "1.5.0", false);
        assert_range("<1 || >=2", "2.0.0", true);
    }

    #[test]
    fn range_tilde_caret() {
        assert_range("~1", "1.9.9", true);
        assert_range("~1", "2.0.0", false);
        assert_range("~1.2", "1.2.9", true);
        assert_range("~1.2", "1.3.0", false);
        assert_range("~1.2.3", "1.2.3", true);
        assert_range("~1.2.3", "1.3.0", false);
        assert_range("^1.2.3", "1.9.9", true);
        assert_range("^1.2.3", "2.0.0", false);
        assert_range("^1.2.3", "1.2.2", false);
        assert_range("^0.1.2", "0.1.9", true);
        assert_range("^0.1.2", "0.2.0", false);
        assert_range("^0.0.1", "0.0.1", true);
        assert_range("^0.0.1", "0.0.2", false);
        assert_range("^0.0", "0.0.9", true);
        assert_range("^0.0", "0.1.0", false);
        assert_range("^0", "0.9.9", true);
        assert_range("^0", "1.0.0", false);
    }

    #[test]
    fn range_latest() {
        for (range, expected) in [
            ("*", true),
            (">=3.1", true),
            ("<=5.6", false),
            ("<6", false),
            ("^5", false),
            ("~5.4", false),
        ] {
            assert_eq!(
                VersionRange::parse(range).unwrap().test(&Version::LATEST),
                expected,
                "{range}"
            );
        }
    }
}
