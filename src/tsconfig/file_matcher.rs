//! File matching logic for tsconfig include/exclude/files patterns.
//!
//! Based on vite-tsconfig-paths implementation:
//! <https://github.com/aleclarson/vite-tsconfig-paths>

use std::path::{Path, PathBuf};

/// Matches files against tsconfig include/exclude/files patterns.
///
/// Implements the matching logic from vite-tsconfig-paths which uses globrex
/// for pattern compilation. This implementation uses fast-glob instead.
///
/// ## Matching Rules
///
/// 1. **Files priority**: If a file is in the `files` array, it's included regardless of exclude patterns
/// 2. **Include matching**: File must match at least one include pattern
/// 3. **Exclude filtering**: File must NOT match any exclude pattern
///
/// ## Default Values
///
/// - Include: `["**/*"]` if not specified
/// - Exclude: `["node_modules", "bower_components", "jspm_packages"]` + outDir if specified
#[derive(Debug)]
pub struct TsconfigFileMatcher {
    /// Explicit files (highest priority, overrides exclude)
    files: Option<Vec<String>>,

    /// Include patterns (defaults to `["**/*"]`)
    include_patterns: Vec<String>,

    /// Exclude patterns (defaults to node_modules, bower_components, jspm_packages)
    exclude_patterns: Vec<String>,

    /// Directory containing tsconfig.json
    tsconfig_dir: PathBuf,
}

impl TsconfigFileMatcher {
    /// Create a matcher that matches nothing (for empty files + no include case).
    ///
    /// Per vite-tsconfig-paths: when `files` is explicitly empty AND `include` is
    /// missing or empty, the tsconfig should not match any files.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            files: None,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            tsconfig_dir: PathBuf::new(),
        }
    }

    /// Create matcher from tsconfig fields.
    ///
    /// # Arguments
    ///
    /// * `files` - Explicit files array from tsconfig
    /// * `include` - Include patterns from tsconfig
    /// * `exclude` - Exclude patterns from tsconfig
    /// * `out_dir` - CompilerOptions.outDir (automatically added to exclude)
    /// * `tsconfig_dir` - Directory containing tsconfig.json
    #[must_use]
    pub fn new(
        files: Option<Vec<String>>,
        include: Option<Vec<String>>,
        exclude: Option<Vec<String>>,
        out_dir: Option<&Path>,
        tsconfig_dir: PathBuf,
    ) -> Self {
        // Default include: **/* (only if include not specified AND files not specified)
        // If files is specified without include, don't default include
        let include_patterns = include.unwrap_or_else(|| {
            if files.is_some() {
                Vec::new() // No default include when files is specified
            } else {
                vec!["**/*".to_string()]
            }
        });

        // Start with default excludes
        let mut exclude_patterns = vec![
            "node_modules".to_string(),
            "bower_components".to_string(),
            "jspm_packages".to_string(),
        ];

        // Merge user-specified excludes with defaults
        if let Some(user_excludes) = exclude {
            exclude_patterns.extend(user_excludes);
        }

        // Add outDir to exclude if specified
        if let Some(out_dir) = out_dir {
            if let Some(out_dir_str) = out_dir.to_str() {
                exclude_patterns.push(out_dir_str.to_string());
            }
        }

        Self {
            files,
            include_patterns: Self::normalize_patterns(include_patterns, &tsconfig_dir),
            exclude_patterns: Self::normalize_patterns(exclude_patterns, &tsconfig_dir),
            tsconfig_dir,
        }
    }

    /// Normalize patterns per vite-tsconfig-paths logic.
    ///
    /// Rules:
    /// 1. Convert absolute paths to relative from tsconfig_dir
    /// 2. Ensure patterns start with ./
    /// 3. Expand non-glob patterns: "foo" → `["./foo/**"]`
    /// 4. File-like patterns: "foo.ts" → `["./foo.ts", "./foo.ts/**"]`
    fn normalize_patterns(patterns: Vec<String>, tsconfig_dir: &Path) -> Vec<String> {
        patterns
            .into_iter()
            .flat_map(|#[cfg_attr(not(target_os = "windows"), allow(unused_mut))] mut pattern| {
                // On Windows, convert to lowercase for case-insensitive matching
                #[cfg(target_os = "windows")]
                {
                    pattern = pattern.to_lowercase();
                }
                // Convert absolute to relative
                #[allow(clippy::option_if_let_else)] // map_or would cause borrow checker issues
                let pattern = if Path::new(&pattern).is_absolute() {
                    match Path::new(&pattern).strip_prefix(tsconfig_dir) {
                        Ok(rel) => rel.to_string_lossy().to_string(),
                        Err(_) => pattern,
                    }
                } else {
                    pattern
                };

                // Ensure starts with ./
                let pattern = if pattern.starts_with("./") || pattern.starts_with("../") {
                    pattern
                } else {
                    format!("./{pattern}")
                };

                // Handle non-glob patterns
                let ends_with_glob = pattern
                    .split('/')
                    .next_back()
                    .is_some_and(|part| part.contains('*') || part.contains('?'));

                if ends_with_glob {
                    // Pattern already has wildcards, use as-is
                    vec![pattern]
                } else {
                    // Non-glob pattern: expand to match directory
                    // Strip trailing slash before adding /**
                    let pattern_base = pattern.trim_end_matches('/');
                    let mut patterns = Vec::new();

                    // If looks like a file (has extension after last slash), also match exact
                    if pattern
                        .rsplit('/')
                        .next()
                        .is_some_and(|part| part.contains('.') && part != "." && part != "..")
                    {
                        patterns.push(pattern.clone());
                    }

                    patterns.push(format!("{pattern_base}/**"));
                    patterns
                }
            })
            .collect()
    }

    /// Test if a file matches this tsconfig's patterns.
    ///
    /// # Returns
    ///
    /// `true` if the file matches, `false` otherwise.
    ///
    /// # Algorithm
    ///
    /// 1. Normalize the file path (relative to tsconfig_dir with ./ prefix)
    /// 2. Check files array first (highest priority, overrides exclude)
    /// 3. Check if path matches any include pattern
    /// 4. Check if path matches any exclude pattern
    #[must_use]
    pub fn matches(&self, file_path: &Path) -> bool {
        // Normalize path for matching
        #[allow(clippy::manual_let_else)] // Match is clearer here
        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut normalized = match self.normalize_path(file_path) {
            Some(p) => p,
            None => return false, // Path can't be normalized
        };

        // On Windows, convert to lowercase for case-insensitive matching
        #[cfg(target_os = "windows")]
        {
            normalized = normalized.to_lowercase();
        }

        // 1. Check files array first (absolute priority)
        if let Some(files) = &self.files {
            for file in files {
                // Check both exact match and ends_with
                if normalized == *file || normalized.ends_with(file) {
                    return true; // Files overrides exclude
                }
            }
            // If files specified but no match, continue to include/exclude
            // (unless include is empty)
            if self.include_patterns.is_empty() {
                return false;
            }
        }

        // 2. Check if empty patterns (match nothing case)
        if self.include_patterns.is_empty() {
            return false;
        }

        // 3. Test against include patterns
        let mut included = false;
        for pattern in &self.include_patterns {
            if fast_glob::glob_match(pattern, &normalized) {
                included = true;
                break;
            }
        }

        if !included {
            return false;
        }

        // 4. Test against exclude patterns
        for pattern in &self.exclude_patterns {
            if fast_glob::glob_match(pattern, &normalized) {
                return false;
            }
        }

        true
    }

    /// Normalize file path for matching.
    ///
    /// Rules (from vite-tsconfig-paths):
    /// 1. Remove query parameters (e.g., `?inline`)
    /// 2. Convert to absolute if relative
    /// 3. Make relative to tsconfig_dir
    /// 4. Use forward slashes for cross-platform consistency
    /// 5. Prepend ./ if needed
    ///
    /// # Returns
    ///
    /// `None` if path is outside tsconfig_dir or can't be normalized.
    fn normalize_path(&self, file_path: &Path) -> Option<String> {
        // Remove query parameters
        let path_str = file_path.to_str()?;
        let path_str = path_str.split('?').next()?;
        let file_path = Path::new(path_str);

        // Make absolute if relative
        let absolute = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(file_path)
        };

        // Make relative to tsconfig directory
        let relative = absolute.strip_prefix(&self.tsconfig_dir).ok()?;

        // Convert to string with forward slashes
        let mut normalized = relative.to_str()?.replace('\\', "/");

        // Ensure starts with ./
        if !normalized.starts_with("./") && !normalized.starts_with("../") {
            normalized = format!("./{normalized}");
        }

        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_matcher() {
        let matcher = TsconfigFileMatcher::empty();
        let path = PathBuf::from("index.ts");
        assert!(!matcher.matches(&path));
    }

    #[test]
    fn test_normalize_patterns() {
        let tsconfig_dir = PathBuf::from("/project");
        let patterns = vec!["src/**/*.ts".to_string(), "lib".to_string(), "file.ts".to_string()];

        let normalized = TsconfigFileMatcher::normalize_patterns(patterns, &tsconfig_dir);

        assert_eq!(normalized, vec!["./src/**/*.ts", "./lib/**", "./file.ts", "./file.ts/**",]);
    }
}
