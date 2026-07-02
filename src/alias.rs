use std::{borrow::Cow, path::Path};

use compact_str::CompactString;

use crate::{
    Alias, AliasValue, CachedPath, ResolveError, ResolverImpl, TsConfig,
    context::ResolveContext as Ctx,
    path::{PathUtil, SLASH_START},
};

#[derive(Clone, Default)]
pub struct CompiledAlias {
    entries: Vec<CompiledAliasEntry>,
    /// First byte of each entry's key (or wildcard prefix), parallel to `entries`. Packed
    /// contiguously so the per-specifier scan touches one or two cache lines instead of striding
    /// over the ~100-byte entries. `0` marks an entry that must always be evaluated (an empty
    /// key/prefix can match any specifier); a key genuinely starting with NUL also maps to `0`,
    /// which just skips its fast-reject.
    first_bytes: Box<[u8]>,
    /// 256-bit set of `first_bytes` values for an O(1) "no entry can match" exit.
    byte_mask: [u64; 4],
}

impl CompiledAlias {
    fn mask_contains(&self, byte: u8) -> bool {
        self.byte_mask[usize::from(byte >> 6)] & (1u64 << (byte & 63)) != 0
    }

    /// Whether any entry's first byte is compatible with `specifier`, without touching the
    /// entries. `0` in the mask means an always-evaluated entry exists.
    fn may_match(&self, specifier: &[u8]) -> bool {
        self.mask_contains(0) || specifier.first().is_some_and(|&byte| self.mask_contains(byte))
    }

    /// Entries whose first byte is compatible with `specifier`, in declaration order. This is
    /// only the fast-reject; callers still confirm with [`CompiledAliasEntry::key_matches`].
    fn matching<'a>(&'a self, specifier: &'a [u8]) -> impl Iterator<Item = &'a CompiledAliasEntry> {
        let first = specifier.first().copied();
        self.first_bytes
            .iter()
            .zip(&self.entries)
            .filter(move |&(&byte, _)| byte == 0 || Some(byte) == first)
            .map(|(_, entry)| entry)
    }

    /// Whether any entry's key matches `specifier` (raw bytes), gated by the first-byte mask.
    /// This runs for every file candidate when `alias` is configured, so on the common path
    /// (no always-evaluated entry) it jumps straight to candidate entries with `memchr` instead
    /// of scanning `first_bytes` with a branch per entry.
    pub(crate) fn any_key_matches(&self, specifier: &[u8]) -> bool {
        if !self.may_match(specifier) {
            return false;
        }
        if self.mask_contains(0) {
            return self.matching(specifier).any(|entry| entry.key_matches(specifier));
        }
        // `may_match` returned true without a `0` entry, so the specifier is non-empty.
        let Some(&first) = specifier.first() else {
            return false;
        };
        memchr::memchr_iter(first, &self.first_bytes)
            .any(|index| self.entries[index].key_matches(specifier))
    }
}

#[derive(Clone)]
pub struct CompiledAliasEntry {
    key: CompactString,
    match_kind: AliasMatchKind,
    specifiers: Vec<AliasValue>,
    /// First byte of the key (or wildcard prefix), cached so the alias loop can fast-reject
    /// non-matching entries without dispatching into the match-kind branch. `None` means an
    /// empty key/prefix — which can match any specifier (e.g. a `*` wildcard) — so the entry
    /// is always evaluated.
    match_first_byte: Option<u8>,
}

#[derive(Clone)]
pub enum AliasMatchKind {
    Exact,
    Prefix,
    Wildcard { prefix: CompactString, suffix: CompactString },
}

pub fn compile_alias(aliases: &Alias) -> CompiledAlias {
    let entries: Vec<CompiledAliasEntry> = aliases
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
            let match_first_byte = match &match_kind {
                AliasMatchKind::Exact | AliasMatchKind::Prefix => key.as_bytes().first().copied(),
                AliasMatchKind::Wildcard { prefix, .. } => prefix.as_bytes().first().copied(),
            };
            CompiledAliasEntry { key, match_kind, specifiers: specifiers.clone(), match_first_byte }
        })
        .collect();
    let first_bytes: Box<[u8]> =
        entries.iter().map(|entry| entry.match_first_byte.unwrap_or(0)).collect();
    let mut byte_mask = [0u64; 4];
    for &byte in &first_bytes {
        byte_mask[usize::from(byte >> 6)] |= 1u64 << (byte & 63);
    }
    CompiledAlias { entries, first_bytes, byte_mask }
}

impl CompiledAliasEntry {
    /// Whether this entry's key matches `specifier` (raw bytes). Matching on bytes lets a caller
    /// gate on a path's `OsStr` without paying for UTF-8 validation up front.
    pub(crate) fn key_matches(&self, specifier: &[u8]) -> bool {
        // Fast-reject on the required first byte; `None` matches any specifier.
        if let Some(required) = self.match_first_byte
            && Some(required) != specifier.first().copied()
        {
            return false;
        }
        match &self.match_kind {
            AliasMatchKind::Exact => self.key.as_bytes() == specifier,
            // Prefix/suffix is validated later in `load_alias_value`.
            AliasMatchKind::Wildcard { .. } => true,
            AliasMatchKind::Prefix => specifier
                .strip_prefix(self.key.as_bytes())
                .is_some_and(|tail| tail.is_empty() || matches!(tail.first(), Some(b'/' | b'\\'))),
        }
    }
}

impl ResolverImpl {
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
        if !aliases.may_match(specifier.as_bytes()) {
            return Ok(None);
        }
        for alias in aliases.matching(specifier.as_bytes()) {
            if !alias.key_matches(specifier.as_bytes()) {
                continue;
            }
            let alias_key = alias.key.as_str();
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
                        if self.is_file(&cached_alias_path, ctx) {
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
