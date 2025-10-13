use serde_json::Value as JSONValue;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    pub raw: String,
}

impl VersionRange {
    #[must_use]
    pub fn new(raw: String) -> Self {
        Self { raw }
    }

    #[must_use]
    pub fn matches(&self, version: &str) -> bool {
        if self.raw == "*" {
            return true;
        }

        if let Some(min_version) = self.raw.strip_prefix(">=") {
            return compare_versions(version, min_version.trim()) >= 0;
        }

        if let Some(exact_version) = self.raw.strip_prefix('=') {
            return version == exact_version.trim();
        }

        version == self.raw
    }
}

fn compare_versions(v1: &str, v2: &str) -> i32 {
    let parts1: Vec<u32> = v1.split('.').filter_map(|p| p.parse().ok()).collect();
    let parts2: Vec<u32> = v2.split('.').filter_map(|p| p.parse().ok()).collect();

    for i in 0..parts1.len().max(parts2.len()) {
        let p1 = parts1.get(i).copied().unwrap_or(0);
        let p2 = parts2.get(i).copied().unwrap_or(0);

        if p1 < p2 {
            return -1;
        }
        if p1 > p2 {
            return 1;
        }
    }

    0
}

#[derive(Debug, Clone)]
pub struct TypesVersions {
    pub version_mappings: Vec<(VersionRange, HashMap<String, Vec<String>>)>,
}

impl TypesVersions {
    #[must_use]
    pub fn new(version_mappings: Vec<(VersionRange, HashMap<String, Vec<String>>)>) -> Self {
        Self { version_mappings }
    }

    pub fn from_json(json: &serde_json::Map<String, JSONValue>) -> Option<Self> {
        let mut version_mappings = Vec::new();

        for (version_range_str, mappings_value) in json {
            let version_range = VersionRange::new(version_range_str.clone());

            if let Some(mappings_obj) = mappings_value.as_object() {
                let mut mappings = HashMap::new();

                for (pattern, paths_value) in mappings_obj {
                    if let Some(paths_array) = paths_value.as_array() {
                        let paths: Vec<String> = paths_array
                            .iter()
                            .filter_map(JSONValue::as_str)
                            .map(ToString::to_string)
                            .collect();

                        if !paths.is_empty() {
                            mappings.insert(pattern.clone(), paths);
                        }
                    }
                }

                if !mappings.is_empty() {
                    version_mappings.push((version_range, mappings));
                }
            }
        }

        if version_mappings.is_empty() { None } else { Some(Self::new(version_mappings)) }
    }

    #[must_use]
    pub fn resolve_for_version(
        &self,
        typescript_version: &str,
        subpath: &str,
    ) -> Option<Vec<String>> {
        for (range, mappings) in &self.version_mappings {
            if range.matches(typescript_version) {
                if let Some(paths) = mappings.get(subpath) {
                    return Some(paths.clone());
                }
                if let Some(paths) = mappings.get("*") {
                    let resolved: Vec<String> =
                        paths.iter().map(|p| p.replace('*', subpath)).collect();
                    return Some(resolved);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_range_matches() {
        let range = VersionRange::new(">=4.2".to_string());
        assert!(range.matches("4.2"));
        assert!(range.matches("4.3"));
        assert!(range.matches("5.0"));
        assert!(!range.matches("4.1"));
        assert!(!range.matches("3.9"));
    }

    #[test]
    fn test_version_range_wildcard() {
        let range = VersionRange::new("*".to_string());
        assert!(range.matches("3.0"));
        assert!(range.matches("4.2"));
        assert!(range.matches("5.0"));
    }

    #[test]
    fn test_types_versions_resolve() {
        let mut mappings_42 = HashMap::new();
        mappings_42.insert("*".to_string(), vec!["ts4.2/*".to_string()]);

        let mut mappings_37 = HashMap::new();
        mappings_37.insert("*".to_string(), vec!["ts3.7/*".to_string()]);

        let mut mappings_default = HashMap::new();
        mappings_default.insert("*".to_string(), vec!["ts3.0/*".to_string()]);

        let types_versions = TypesVersions::new(vec![
            (VersionRange::new(">=4.2".to_string()), mappings_42),
            (VersionRange::new(">=3.7".to_string()), mappings_37),
            (VersionRange::new("*".to_string()), mappings_default),
        ]);

        let result = types_versions.resolve_for_version("4.3", "index");
        assert_eq!(result, Some(vec!["ts4.2/index".to_string()]));

        let result = types_versions.resolve_for_version("3.8", "index");
        assert_eq!(result, Some(vec!["ts3.7/index".to_string()]));

        let result = types_versions.resolve_for_version("3.0", "index");
        assert_eq!(result, Some(vec!["ts3.0/index".to_string()]));
    }
}
