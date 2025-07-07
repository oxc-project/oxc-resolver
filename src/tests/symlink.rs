#[cfg(target_os = "windows")]
use crate::tests::windows::get_dos_device_path;
#[cfg(target_os = "windows")]
use normalize_path::NormalizePath;
use std::path::PathBuf;
use std::{fs, io, path::Path};

use crate::{ResolveOptions, Resolver};

#[derive(Debug, Clone, Copy)]
enum FileType {
    File,
    Dir,
}

#[allow(unused_variables)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
    file_type: FileType,
) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(target_os = "windows")]
    match file_type {
        // NOTE: original path should use `\` instead of `/` for relative paths
        //       otherwise the symlink will be broken and the test will fail with InvalidFilename error
        FileType::File => std::os::windows::fs::symlink_file(original.as_ref().normalize(), link),
        FileType::Dir => std::os::windows::fs::symlink_dir(original.as_ref().normalize(), link),
    }
}

fn init(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    if temp_path.exists() {
        _ = fs::remove_dir_all(temp_path);
    }
    fs::create_dir(temp_path)?;
    symlink(dirname.join("../lib/index.js"), temp_path.join("test"), FileType::File)?;
    symlink(dirname.join("../lib"), temp_path.join("test2"), FileType::Dir)?;
    fs::remove_dir_all(temp_path)
}

fn create_symlinks(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    fs::create_dir(temp_path)?;
    symlink(
        dirname.join("../lib/index.js").canonicalize()?,
        temp_path.join("index.js"),
        FileType::File,
    )?;
    symlink(dirname.join("../lib").canonicalize().unwrap(), temp_path.join("lib"), FileType::Dir)?;
    symlink(dirname.join("..").canonicalize().unwrap(), temp_path.join("this"), FileType::Dir)?;
    symlink(temp_path.join("this"), temp_path.join("that"), FileType::Dir)?;
    symlink(Path::new("../../lib/index.js"), temp_path.join("node.relative.js"), FileType::File)?;
    symlink(
        Path::new("./node.relative.js"),
        temp_path.join("node.relative.sym.js"),
        FileType::File,
    )?;

    #[cfg(target_os = "windows")]
    {
        // Ideally we should point to a Volume that does not have a drive letter.
        // However, it's not trivial to create a Volume in CI environment.
        // Here we are just picking up any Volume, as resolver itself is not calling `fs::canonicalize`,
        // which potentially can resolve the Volume GUID into driver letter whenever possible.
        let dos_device_temp_path = get_dos_device_path(temp_path).unwrap();
        symlink(
            dos_device_temp_path.join(r"..\..\lib"),
            temp_path.join("device_path_lib"),
            FileType::Dir,
        )?;
        symlink(
            dos_device_temp_path.join(r"..\..\lib\index.js"),
            temp_path.join("device_path_index.js"),
            FileType::File,
        )?;
    }

    Ok(())
}

fn cleanup_symlinks(temp_path: &Path) {
    _ = fs::remove_dir_all(temp_path);
}

struct SymlinkFixturePaths {
    root: PathBuf,
    temp_path: PathBuf,
}

/// Prepares symlinks for the test.
/// Specify a different `temp_path_segment` for each test to avoid conflicts when tests are executed concurrently.
/// Returns `Ok(None)` if the symlink fixtures cannot be created at all (usually due to a lack of permission).
/// Returns `Ok(Some(_))` if the symlink fixtures are created successfully, or already exist.
/// Returns `Err(_)` if there is error creating the symlinks.
fn prepare_symlinks<P: AsRef<Path>>(
    temp_path_segment: P,
) -> io::Result<Option<SymlinkFixturePaths>> {
    let root = super::fixture_root().join("enhanced_resolve");
    let dirname = root.join("test");
    let temp_path = dirname.join(temp_path_segment.as_ref());
    if !temp_path.exists() {
        if let Err(err) = init(&dirname, &temp_path) {
            println!(
                "Skipped test: Failed to create symlinks. You may need administrator privileges. Error: {err}"
            );
            return Ok(None);
        }
        if let Err(err) = create_symlinks(&dirname, &temp_path) {
            cleanup_symlinks(&temp_path);
            return Err(err);
        }
    }

    Ok(Some(SymlinkFixturePaths { root, temp_path }))
}

#[test]
fn test() {
    let Some(SymlinkFixturePaths { root, temp_path }) = prepare_symlinks("temp").unwrap() else {
        return;
    };
    let resolver_without_symlinks =
        Resolver::new(ResolveOptions { symlinks: false, ..ResolveOptions::default() });
    let resolver_with_symlinks = Resolver::default();

    #[rustfmt::skip]
    let pass = [
        ("with a symlink to a file", temp_path.clone(), "./index.js"),
        ("with a relative symlink to a file", temp_path.clone(), "./node.relative.js"),
        ("with a relative symlink to a symlink to a file", temp_path.clone(), "./node.relative.sym.js"),
        ("with a symlink to a directory 1", temp_path.clone(), "./lib/index.js"),
        ("with a symlink to a directory 2", temp_path.clone(), "./this/lib/index.js"),
        ("with multiple symlinks in the path 1", temp_path.clone(), "./this/test/temp/index.js"),
        ("with multiple symlinks in the path 2", temp_path.clone(), "./this/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3", temp_path.clone(), "./this/test/temp/this/lib/index.js"),
        ("with a symlink to a directory 2 (chained)", temp_path.clone(), "./that/lib/index.js"),
        ("with multiple symlinks in the path 1 (chained)", temp_path.clone(), "./that/test/temp/index.js"),
        ("with multiple symlinks in the path 2 (chained)", temp_path.clone(), "./that/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3 (chained)", temp_path.clone(), "./that/test/temp/that/lib/index.js"),
        ("with symlinked directory as context 1", temp_path.join( "lib"), "./index.js"),
        ("with symlinked directory as context 2", temp_path.join( "this"), "./lib/index.js"),
        ("with symlinked directory as context and in path", temp_path.join( "this"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path", temp_path.join( "this/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file", temp_path.join( "this/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory", temp_path.join( "this/test"), "./temp/lib/index.js"),
        ("with symlinked directory as context 2 (chained)", temp_path.join( "that"), "./lib/index.js"),
        ("with symlinked directory as context and in path (chained)", temp_path.join( "that"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path (chained)", temp_path.join( "that/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file (chained)", temp_path.join( "that/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory (chained)", temp_path.join( "that/test"), "./temp/lib/index.js")
    ];

    for (comment, path, request) in pass {
        let filename = resolver_with_symlinks.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(filename, Ok(root.join("lib/index.js")), "{comment:?}");

        let resolved_path =
            resolver_without_symlinks.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(path.join(request)));
    }
}

#[cfg(target_os = "windows")]
#[test]
fn test_unsupported_targets() {
    use crate::ResolveError;

    let Some(SymlinkFixturePaths { root: _, temp_path }) =
        prepare_symlinks("temp.test_unsupported_targets").unwrap()
    else {
        return;
    };
    let resolver_with_symlinks = Resolver::default();

    // Symlinks pointing to unsupported DOS device paths are not followed, as if `symlinks = false`.
    // See doc of `ResolveOptions::symlinks` for details.
    // They are treated as if they are ordinary files and folders.
    assert_eq!(
        resolver_with_symlinks.resolve(&temp_path, "./device_path_lib").unwrap().full_path(),
        temp_path.join("device_path_lib/index.js"),
    );
    assert_eq!(
        resolver_with_symlinks.resolve(&temp_path, "./device_path_index.js").unwrap().full_path(),
        temp_path.join("device_path_index.js"),
    );

    // UB if the resolution starts at a directory with unsupported DOS device path. Don't do this.
    // While we haven't set up any convention on this, de facto behavior for now is
    // * if there is `package.json` in the ancestor, a `ResolveError::PathNotSupported` will be returned
    //   from `FsCachedPath::find_package_json` when trying to canonicalize the full path of `package.json`.
    // * Otherwise, a `ResolveError::NotFound` will be returned.
    let dos_device_temp_path = get_dos_device_path(&temp_path).unwrap();
    assert_eq!(
        resolver_with_symlinks.resolve(&dos_device_temp_path, "./index.js"),
        Err(ResolveError::PathNotSupported(dos_device_temp_path))
    );
}
