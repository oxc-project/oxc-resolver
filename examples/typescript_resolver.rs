#[cfg(feature = "typescript")]
fn main() {
    use oxc_resolver::{
        ResolveOptions, Resolver,
        typescript::{TypeResolutionMode, TypeScriptOptions},
    };
    use std::path::Path;

    let ts_options = TypeScriptOptions::new()
        .with_typescript_version("5.0.0".to_string())
        .with_type_resolution_mode(TypeResolutionMode::Full)
        .with_resolve_type_references(true);

    let options = ResolveOptions::default().with_typescript_options(ts_options);

    let resolver = Resolver::new(options);

    match resolver.resolve_at_types_package("react", Path::new(".")) {
        Ok(resolution) => {
            println!("Resolved @types/react to: {:?}", resolution.path());
        }
        Err(e) => {
            println!("Failed to resolve @types/react: {e}");
        }
    }

    match resolver.resolve_type_reference_directive(Path::new("./src/index.ts"), "node") {
        Ok(resolution) => {
            println!("Resolved type reference 'node' to: {:?}", resolution.path());
        }
        Err(e) => {
            println!("Failed to resolve type reference 'node': {e}");
        }
    }

    println!("\nTypeScript resolution features:");
    println!("- Type reference directive resolution");
    println!("- @types package resolution with scoped package mangling");
    println!("- typesVersions support");
    println!("- Two-pass resolution strategy");
}

#[cfg(not(feature = "typescript"))]
fn main() {
    println!("This example requires the 'typescript' feature flag.");
    println!("Run with: cargo run --example typescript_resolver --features typescript");
}
