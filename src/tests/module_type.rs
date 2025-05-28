use crate::{ModuleType, ResolveOptions, Resolver};

#[test]
fn test() {
    let f = super::fixture_root().join("misc").join("module-type");

    let resolver = Resolver::new(ResolveOptions { module_type: true, ..ResolveOptions::default() });

    let pass = [
        ("./file.cjs", ModuleType::CommonJs),
        ("./file.mjs", ModuleType::Module),
        ("./file.node", ModuleType::Addon),
        ("./file.wasm", ModuleType::Wasm),
        ("./cjs/file.cjs", ModuleType::CommonJs),
        ("./cjs/file.mjs", ModuleType::Module),
        ("./cjs/file.js", ModuleType::CommonJs),
        ("./esm/file.cjs", ModuleType::CommonJs),
        ("./esm/file.mjs", ModuleType::Module),
        ("./esm/file.js", ModuleType::Module),
    ];

    for (file, module_type) in pass {
        let resolution = resolver.resolve(&f, file).unwrap();
        assert_eq!(resolution.module_type(), Some(module_type), "{file}");
    }

    let fail = ["./file", "./file.ext", "./no/file.js"];

    for file in fail {
        let resolution = resolver.resolve(&f, file).unwrap();
        assert_eq!(resolution.module_type(), None);
    }
}
