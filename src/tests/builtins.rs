use std::path::Path;

use crate::{ResolveError, ResolveOptions, Resolver};

#[test]
fn builtins_off() {
    let f = Path::new("/");
    let resolver = Resolver::default();
    let resolved_path = resolver.resolve(f, "zlib").map(|r| r.full_path());
    assert_eq!(resolved_path, Err(ResolveError::NotFound("zlib".into())));
}

#[test]
fn builtins() {
    let f = Path::new("/");

    let resolver = Resolver::new(ResolveOptions::default().with_builtin_modules(true));

    for request in nodejs_built_in_modules::BUILTINS {
        let prefixed_request = format!("node:{request}");
        for request in [prefixed_request.clone(), request.to_string()] {
            let starts_with_node = request.starts_with("node:");
            let resolved_path = resolver.resolve(f, &request);
            let err = ResolveError::Builtin {
                resolved: prefixed_request.clone(),
                is_runtime_module: starts_with_node,
            };
            assert_eq!(resolved_path, Err(err), "{request}");
        }
    }

    for request in nodejs_built_in_modules::BUILTINS_WITH_MANDATORY_NODE_PREFIX {
        let resolved_path = resolver.resolve(f, request);
        assert_eq!(resolved_path, Err(ResolveError::NotFound(request.to_string())), "{request}");

        let prefixed_request = format!("node:{request}");
        let resolved_path = resolver.resolve(f, &prefixed_request);
        let err = ResolveError::Builtin { resolved: prefixed_request, is_runtime_module: true };
        assert_eq!(resolved_path, Err(err), "{request}");
    }
}

#[test]
fn fail() {
    let f = Path::new("/");
    let resolver = Resolver::new(ResolveOptions::default().with_builtin_modules(true));
    let request = "xxx";
    let resolved_path = resolver.resolve(f, request);
    let err = ResolveError::NotFound(request.to_string());
    assert_eq!(resolved_path, Err(err), "{request}");
}

#[test]
fn imports() {
    let f = super::fixture().join("builtins");
    let resolver = Resolver::new(ResolveOptions {
        builtin_modules: true,
        condition_names: vec!["node".into()],
        ..ResolveOptions::default()
    });

    for (request, is_runtime_module) in [("#fs", false), ("#http", true)] {
        let resolved_path = resolver.resolve(f.clone(), request).map(|r| r.full_path());
        let err = ResolveError::Builtin {
            resolved: (format!("node:{}", request.trim_start_matches('#'))),
            is_runtime_module,
        };
        assert_eq!(resolved_path, Err(err));
    }
}
