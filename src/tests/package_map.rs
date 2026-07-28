use std::path::{Path, PathBuf};

fn test_package_map(fixture: impl AsRef<Path>, package_map: impl AsRef<Path>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _fixture = root.join(fixture);
    let _package_map = root.join(package_map);
}

#[test]
fn pnpm() {
    test_package_map("fixtures/pnpm", "node_modules/.package-map.json");
}

#[test]
fn yarn() {
    test_package_map("fixtures/yarn", "fixtures/yarn/node_modules/.package-map.json");
}

#[test]
fn pnpm_isolated() {
    test_package_map(
        "fixtures/bench-pm/installs/pnpm-isolated",
        "fixtures/bench-pm/installs/pnpm-isolated/node_modules/.package-map.json",
    );
}

#[test]
fn yarn_isolated() {
    test_package_map(
        "fixtures/bench-pm/installs/yarn-isolated",
        "fixtures/bench-pm/installs/yarn-isolated/node_modules/.package-map.json",
    );
}
