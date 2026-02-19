use std::{env, ffi::OsString};

pub struct NodePath;

impl NodePath {
    pub fn build() -> Vec<String> {
        Self::parse(env::var_os("NODE_PATH"))
    }

    fn parse(node_path: Option<OsString>) -> Vec<String> {
        let Some(node_path) = node_path else {
            return Vec::with_capacity(1);
        };

        let mut entries = env::split_paths(&node_path)
            .filter(|path| path.is_absolute())
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        entries.reserve(1);
        entries
    }
}
