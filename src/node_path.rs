#[cfg(not(target_family = "wasm"))]
use std::{env, ffi::OsString, sync::OnceLock};

#[cfg(not(target_family = "wasm"))]
static NODE_PATH: OnceLock<Vec<String>> = OnceLock::new();

/// `NODE_PATH` support aligned with Node.js module loading docs:
/// <https://nodejs.org/api/modules.html#loading-from-the-global-folders>
pub struct NodePath;

impl NodePath {
    pub fn build() -> &'static [String] {
        #[cfg(target_family = "wasm")]
        {
            &[]
        }
        #[cfg(not(target_family = "wasm"))]
        {
            NODE_PATH.get_or_init(|| Self::parse(env::var_os("NODE_PATH"))).as_slice()
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn parse(node_path: Option<OsString>) -> Vec<String> {
        let Some(node_path) = node_path else {
            return Vec::new();
        };

        env::split_paths(&node_path)
            .filter(|path| path.is_absolute())
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    }
}
