use std::path::{Path, PathBuf};

use crate::Resolution;

#[test]
fn test() {
    let resolution = Resolution {
        path: PathBuf::from("foo"),
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
