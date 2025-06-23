use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct TsconfigResolveContext {
    extended_configs: Vec<PathBuf>,
}

impl TsconfigResolveContext {
    pub fn with_extended_file<R, T: FnOnce(&mut Self) -> R>(&mut self, path: PathBuf, cb: T) -> R {
        self.extended_configs.push(path);
        let result = cb(self);
        self.extended_configs.pop();
        result
    }

    pub fn is_already_extended(&self, path: &Path) -> bool {
        self.extended_configs.iter().any(|config| config == path)
    }

    pub fn get_extended_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut new_vec = Vec::with_capacity(self.extended_configs.len() + 1);
        new_vec.extend_from_slice(&self.extended_configs);
        new_vec.push(path);
        new_vec
    }
}
