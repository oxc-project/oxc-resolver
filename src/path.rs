//! Path Utilities
//!
//! Code adapted from the following libraries
//! * [path-absolutize](https://docs.rs/path-absolutize)
//! * [normalize_path](https://docs.rs/normalize-path)
use std::path::{Component, Path, PathBuf};

pub const SLASH_START: &[char; 2] = &['/', '\\'];

/// Extension trait to add path normalization to std's [`Path`].
pub trait PathUtil {
    /// Normalize this path without performing I/O.
    ///
    /// All redundant separator and up-level references are collapsed.
    ///
    /// However, this does not resolve links.
    fn normalize(&self) -> PathBuf;

    /// Like `normalize`, but don't require the path to be absolute.
    fn normalize_relative(&self) -> PathBuf;

    /// Normalize with subpath assuming this path is normalized without performing I/O.
    ///
    /// All redundant separator and up-level references are collapsed.
    ///
    /// However, this does not resolve links.
    fn normalize_with<P: AsRef<Path>>(&self, subpath: P) -> PathBuf;

    /// Defined in ESM PACKAGE_TARGET_RESOLVE
    /// If target split on "/" or "\" contains any "", ".", "..", or "node_modules" segments after the first "." segment, case insensitive and including percent encoded variants
    fn is_invalid_exports_target(&self) -> bool;
}

impl PathUtil for Path {
    // https://github.com/parcel-bundler/parcel/blob/e0b99c2a42e9109a9ecbd6f537844a1b33e7faf5/packages/utils/node-resolver-rs/src/path.rs#L7
    fn normalize(&self) -> PathBuf {
        let mut components = self.components().peekable();
        let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek() {
            let buf = PathBuf::from(c.as_os_str());
            components.next();
            buf
        } else {
            PathBuf::new()
        };

        for component in components {
            match component {
                Component::Prefix(..) => unreachable!("Path {:?}", self),
                Component::RootDir => {
                    ret.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    ret.pop();
                }
                Component::Normal(c) => {
                    ret.push(c);
                }
            }
        }

        ret
    }

    fn normalize_relative(&self) -> PathBuf {
        let mut normalized = PathBuf::new();
        for comp in self.components() {
            match comp {
                Component::ParentDir => {
                    if !normalized.pop() {
                        normalized.push(Component::ParentDir);
                    }
                }
                Component::CurDir => {}
                comp => normalized.push(comp),
            }
        }
        normalized
    }

    // https://github.com/parcel-bundler/parcel/blob/e0b99c2a42e9109a9ecbd6f537844a1b33e7faf5/packages/utils/node-resolver-rs/src/path.rs#L37
    fn normalize_with<B: AsRef<Self>>(&self, subpath: B) -> PathBuf {
        let subpath = subpath.as_ref();

        let mut components = subpath.components();

        let Some(head) = components.next() else { return subpath.to_path_buf() };

        if matches!(head, Component::Prefix(..) | Component::RootDir) {
            return subpath.to_path_buf();
        }

        let mut ret = self.to_path_buf();
        for component in std::iter::once(head).chain(components) {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    ret.pop();
                }
                Component::Normal(c) => {
                    ret.push(c);
                }
                Component::Prefix(..) | Component::RootDir => {
                    unreachable!("Path {:?} Subpath {:?}", self, subpath)
                }
            }
        }

        ret
    }

    fn is_invalid_exports_target(&self) -> bool {
        self.components().enumerate().any(|(index, c)| match c {
            Component::ParentDir => true,
            Component::CurDir => index > 0,
            Component::Normal(c) => c.eq_ignore_ascii_case("node_modules"),
            _ => false,
        })
    }
}

// https://github.com/webpack/enhanced-resolve/blob/main/test/path.test.js
#[test]
fn is_invalid_exports_target() {
    let test_cases = [
        "../a.js",
        "../",
        "./a/b/../../../c.js",
        "./a/b/../../../",
        "./../../c.js",
        "./../../",
        "./a/../b/../../c.js",
        "./a/../b/../../",
        "./././../",
    ];

    for case in test_cases {
        assert!(Path::new(case).is_invalid_exports_target(), "{case}");
    }

    assert!(!Path::new("C:").is_invalid_exports_target());
    assert!(!Path::new("/").is_invalid_exports_target());
}

#[test]
fn normalize() {
    assert_eq!(Path::new("/foo/.././foo/").normalize(), Path::new("/foo"));
    assert_eq!(Path::new("C://").normalize(), Path::new("C://"));
    assert_eq!(Path::new("C:").normalize(), Path::new("C:"));
    assert_eq!(Path::new(r"\\server\share").normalize(), Path::new(r"\\server\share"));
}

#[test]
fn normalize_relative() {
    assert_eq!(Path::new("foo/../../foo/").normalize_relative(), Path::new("../foo"));
    assert_eq!(Path::new("foo/.././foo/").normalize_relative(), Path::new("foo"));
    assert_eq!(Path::new("foo../../..").normalize_relative(), Path::new(".."));
    assert_eq!(Path::new("jest-runner-../../").normalize_relative(), Path::new(""));
}
