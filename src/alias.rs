use std::{borrow::Cow, path::Path};

use compact_str::CompactString;

use crate::{
    Alias, AliasValue, CachedPath, FileSystem, ResolveError, ResolverGeneric, TsConfig,
    context::ResolveContext as Ctx,
    path::{PathUtil, SLASH_START},
};

pub type CompiledAlias = Vec<CompiledAliasEntry>;

#[derive(Clone)]
pub struct CompiledAliasEntry {
    key: CompactString,
    match_kind: AliasMatchKind,
    specifiers: Vec<AliasValue>,
}

#[derive(Clone)]
pub enum AliasMatchKind {
    Exact,
    Prefix,
    Wildcard { prefix: CompactString, suffix: CompactString },
}

pub fn compile_alias(aliases: &Alias) -> CompiledAlias {
    aliases
        .iter()
        .map(|(key, specifiers)| {
            let (key, match_kind) = key.strip_suffix('$').map_or_else(
                || {
                    if let Some((prefix, suffix)) = key.split_once('*') {
                        (
                            CompactString::new(key),
                            AliasMatchKind::Wildcard {
                                prefix: CompactString::new(prefix),
                                suffix: CompactString::new(suffix),
                            },
                        )
                    } else {
                        (CompactString::new(key), AliasMatchKind::Prefix)
                    }
                },
                |stripped_key| (CompactString::new(stripped_key), AliasMatchKind::Exact),
            );
            CompiledAliasEntry { key, match_kind, specifiers: specifiers.clone() }
        })
        .collect()
}

impl<Fs: FileSystem> ResolverGeneric<Fs> {
    pub(super) fn load_alias_by_options(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        aliases: &Alias,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<Option<CachedPath>, ResolveError> {
        let compiled_aliases = compile_alias(aliases);
        self.load_alias(cached_path, specifier, &compiled_aliases, tsconfig, ctx)
    }

    /// enhanced-resolve: AliasPlugin for [crate::ResolveOptions::alias] and [crate::ResolveOptions::fallback].
    pub(super) fn load_alias(
        &self,
        cached_path: &CachedPath,
        specifier: &str,
        aliases: &CompiledAlias,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
    ) -> Result<Option<CachedPath>, ResolveError> {
        for alias in aliases {
            let alias_key = alias.key.as_str();
            match &alias.match_kind {
                AliasMatchKind::Exact => {
                    if alias_key != specifier {
                        continue;
                    }
                }
                AliasMatchKind::Wildcard { .. } => {}
                AliasMatchKind::Prefix => {
                    if Self::strip_package_name(specifier, alias_key).is_none() {
                        continue;
                    }
                }
            }
            // It should stop resolving when all of the tried alias values
            // failed to resolve.
            // <https://github.com/webpack/enhanced-resolve/blob/570337b969eee46120a18b62b72809a3246147da/lib/AliasPlugin.js#L65>
            let mut should_stop = false;
            for r in &alias.specifiers {
                match r {
                    AliasValue::Path(alias_value) => {
                        if let Some(path) = self.load_alias_value(
                            cached_path,
                            alias_key,
                            &alias.match_kind,
                            alias_value,
                            specifier,
                            tsconfig,
                            ctx,
                            &mut should_stop,
                        )? {
                            return Ok(Some(path));
                        }
                    }
                    AliasValue::Ignore => {
                        let cached_path = cached_path.normalize_with(alias_key, &self.cache);
                        return Err(ResolveError::Ignored(cached_path.to_path_buf()));
                    }
                }
            }
            if should_stop {
                return Err(ResolveError::MatchedAliasNotFound(
                    specifier.to_string(),
                    alias_key.to_string(),
                ));
            }
        }
        Ok(None)
    }

    fn load_alias_value(
        &self,
        cached_path: &CachedPath,
        alias_key: &str,
        alias_match_kind: &AliasMatchKind,
        alias_value: &str,
        request: &str,
        tsconfig: Option<&TsConfig>,
        ctx: &mut Ctx,
        should_stop: &mut bool,
    ) -> Result<Option<CachedPath>, ResolveError> {
        if request != alias_value
            && !request.strip_prefix(alias_value).is_some_and(|prefix| prefix.starts_with('/'))
        {
            let new_specifier = match alias_match_kind {
                AliasMatchKind::Wildcard { prefix, suffix } => {
                    // Resolve wildcard, e.g. `@/*` -> `./src/*`
                    let Some(alias_key) = request
                        .strip_prefix(prefix.as_str())
                        .and_then(|specifier| specifier.strip_suffix(suffix.as_str()))
                    else {
                        return Ok(None);
                    };
                    if alias_value.contains('*') {
                        Cow::Owned(alias_value.replacen('*', alias_key, 1))
                    } else {
                        Cow::Borrowed(alias_value)
                    }
                }
                AliasMatchKind::Exact | AliasMatchKind::Prefix => {
                    let tail = &request[alias_key.len()..];
                    if tail.is_empty() {
                        Cow::Borrowed(alias_value)
                    } else {
                        let alias_path = Path::new(alias_value).normalize();
                        // Must not append anything to alias_value if it is a file.
                        let cached_alias_path = self.cache.value(&alias_path);
                        if self.cache.is_file(&cached_alias_path, ctx) {
                            return Ok(None);
                        }
                        // Remove the leading slash so the final path is concatenated.
                        let tail = tail.trim_start_matches(SLASH_START);
                        if tail.is_empty() {
                            Cow::Borrowed(alias_value)
                        } else {
                            let normalized = alias_path.normalize_with(tail);
                            Cow::Owned(normalized.to_string_lossy().to_string())
                        }
                    }
                }
            };

            *should_stop = true;
            ctx.with_fully_specified(false);
            return match self.require(cached_path, new_specifier.as_ref(), tsconfig, ctx) {
                Err(ResolveError::NotFound(_) | ResolveError::MatchedAliasNotFound(_, _)) => {
                    Ok(None)
                }
                Ok(path) => Ok(Some(path)),
                Err(err) => Err(err),
            };
        }
        Ok(None)
    }
}
