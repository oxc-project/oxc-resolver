use std::path::{Path, PathBuf};

use crate::{Cache, FileSystem as _, FileSystemOs, Resolution};

#[test]
fn test() {
    #[cfg(feature = "yarn_pnp")]
    let cache = Cache::new(FileSystemOs::new(false));
    #[cfg(not(feature = "yarn_pnp"))]
    let cache = Cache::new(FileSystemOs::new());
    let cached_path = cache.value(Path::new("foo"));
    let resolution = Resolution {
        cached_path,
        query: Some("?query".to_string()),
        fragment: Some("#fragment".to_string()),
        package_json: None,
        module_type: None,
    };
    assert_eq!(resolution.path(), Path::new("foo"));
    assert_eq!(resolution.query(), Some("?query"));
    assert_eq!(resolution.fragment(), Some("#fragment"));
    assert_eq!(resolution.full_path(), PathBuf::from("foo?query#fragment"));
    assert_eq!(resolution.module_type(), None);
    assert_eq!(resolution.into_path_buf(), PathBuf::from("foo"));
}
