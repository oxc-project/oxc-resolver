use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::PathUtil;
use serde::Deserialize;
use typescript_tsconfig_json::{CompilerOptions, TsConfigJson};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfig {
    /// Path to `tsconfig.json`. Contains the `tsconfig.json` filename.
    #[serde(skip)]
    path: PathBuf,

    // Base url for compiler option paths.
    #[serde(skip)]
    paths_base: PathBuf,

    /// The deserialized `tsconfig.json`.
    pub data: TsConfigJson,

    /// Bubbled up project references with a reference to their tsconfig.
    #[serde(default)]
    pub references: Vec<ProjectReference>,
}

/// Project Reference
/// <https://www.typescriptlang.org/docs/handbook/project-references.html>
#[derive(Debug, Deserialize)]
pub struct ProjectReference {
    /// The path property of each reference can point to a directory containing a tsconfig.json file,
    /// or to the config file itself (which may have any name).
    pub path: PathBuf,

    /// Reference to the resolved tsconfig
    #[serde(skip)]
    pub tsconfig: Option<Arc<TsConfig>>,
}

impl TsConfig {
    pub fn parse(path: &Path, json: &mut str) -> Result<Self, serde_json::Error> {
        _ = json_strip_comments::strip(json);

        let data: TsConfigJson = serde_json::from_str(json)?;

        let mut tsconfig = TsConfig {
            path: path.to_path_buf(),
            paths_base: PathBuf::new(),
            references: data.references.as_ref().map_or_else(Vec::new, |refs| {
                refs.iter()
                    .map(|pr| ProjectReference { path: pr.path.clone(), tsconfig: None })
                    .collect()
            }),
            data,
        };
        let directory = tsconfig.directory().to_path_buf();

        if let Some(compiler_options) = &mut tsconfig.data.compiler_options {
            if let Some(base_url) = &compiler_options.base_url {
                compiler_options.base_url = Some(directory.normalize_with(base_url));
            }

            if compiler_options.paths.is_some() {
                tsconfig.paths_base =
                    compiler_options.base_url.as_ref().map_or(directory, Clone::clone);
            }
        }

        Ok(tsconfig)
    }

    /// Directory to `package.json`
    ///
    /// # Panics
    ///
    /// * When the package.json path is misconfigured.
    pub fn directory(&self) -> &Path {
        debug_assert!(self.path.file_name().is_some());
        self.path.parent().unwrap()
    }

    fn base_path(&self) -> &Path {
        self.data
            .compiler_options
            .as_ref()
            .and_then(|opt| opt.base_url.as_ref())
            .map_or_else(|| self.directory(), |path| path.as_ref())
    }

    pub fn extend_tsconfig(&mut self, tsconfig: &Self) {
        if let Some(their_options) = &tsconfig.data.compiler_options {
            let my_options = self.data.compiler_options.get_or_insert(CompilerOptions::default());

            if my_options.paths.is_none() {
                self.paths_base = my_options
                    .base_url
                    .as_ref()
                    .map_or_else(|| tsconfig.paths_base.clone(), Clone::clone);

                my_options.paths = their_options.paths.clone();
            }

            if my_options.base_url.is_none() {
                my_options.base_url = their_options.base_url.clone();
            }
        }
    }

    pub fn resolve(&self, path: &Path, specifier: &str) -> Vec<PathBuf> {
        if path.starts_with(self.base_path()) {
            let paths = self.resolve_path_alias(specifier);
            if !paths.is_empty() {
                return paths;
            }
        }
        for tsconfig in self.references.iter().filter_map(|reference| reference.tsconfig.as_ref()) {
            if path.starts_with(tsconfig.base_path()) {
                return tsconfig.resolve_path_alias(specifier);
            }
        }
        vec![]
    }

    // Copied from parcel
    // <https://github.com/parcel-bundler/parcel/blob/b6224fd519f95e68d8b93ba90376fd94c8b76e69/packages/utils/node-resolver-rs/src/tsconfig.rs#L93>
    pub fn resolve_path_alias(&self, specifier: &str) -> Vec<PathBuf> {
        if specifier.starts_with(|s| s == '/' || s == '.') {
            return vec![];
        }

        let base_url_iter = self
            .data
            .compiler_options
            .as_ref()
            .and_then(|opt| opt.base_url.as_ref())
            .map_or_else(Vec::new, |base_url| vec![base_url.normalize_with(specifier)]);

        let Some(paths_map) =
            self.data.compiler_options.as_ref().and_then(|opt| opt.paths.as_ref())
        else {
            return base_url_iter;
        };

        let paths = paths_map.get(specifier).map_or_else(
            || {
                let mut longest_prefix_length = 0;
                let mut longest_suffix_length = 0;
                let mut best_key: Option<&String> = None;

                for key in paths_map.keys() {
                    if let Some((prefix, suffix)) = key.split_once('*') {
                        if (best_key.is_none() || prefix.len() > longest_prefix_length)
                            && specifier.starts_with(prefix)
                            && specifier.ends_with(suffix)
                        {
                            longest_prefix_length = prefix.len();
                            longest_suffix_length = suffix.len();
                            best_key.replace(key);
                        }
                    }
                }

                best_key.and_then(|key| paths_map.get(key)).map_or_else(Vec::new, |paths| {
                    paths
                        .iter()
                        .map(|path| {
                            path.replace(
                                '*',
                                &specifier[longest_prefix_length
                                    ..specifier.len() - longest_suffix_length],
                            )
                        })
                        .collect::<Vec<_>>()
                })
            },
            Clone::clone,
        );

        paths.into_iter().map(|p| self.paths_base.normalize_with(p)).chain(base_url_iter).collect()
    }
}
