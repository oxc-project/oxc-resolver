use std::sync::Arc;

use crate::{ResolveError, ResolveOptions, Resolver, Restriction};

#[test]
fn should_respect_regexp_restriction() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into()],
		restrictions: vec![Restriction::regex(r"\.(sass|scss|css)$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
	assert_eq!(resolution, Err(ResolveError::NotFound("pck1".to_string())));
}

#[test]
fn should_try_to_find_alternative_1() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_files: vec!["index".into()],
		restrictions: vec![Restriction::regex(r"\.(sass|scss|css)$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_try_to_find_alternative_2() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_fields: vec!["main".into(), "style".into()],
		restrictions: vec![Restriction::regex(r"\.(sass|scss|css)$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck2").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

#[test]
fn should_try_to_find_alternative_3() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into()],
		main_fields: vec!["main".into(), "module".into(), "style".into()],
		restrictions: vec![Restriction::regex(r"\.(sass|scss|css)$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck2").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck2/index.css")));
}

#[test]
fn should_match_regex_restriction_against_normalized_path() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_files: vec!["index".into()],
		restrictions: vec![Restriction::regex(r"/node_modules/pck1/index\.css$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_check_restrictions_in_load_index_with_enforce_extension_disabled() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_files: vec!["index".into()],
		enforce_extension: crate::EnforceExtension::Disabled,
		restrictions: vec![Restriction::regex(r"\.css$").unwrap()],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_apply_multiple_restrictions() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_files: vec!["index".into()],
		restrictions: vec![
			Restriction::regex(r"\.css$").unwrap(),
			Restriction::Fn(Arc::new(|path| {
				path.extension().and_then(|ext| ext.to_str()) != Some("js")
			})),
		],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1").map(|r| r.full_path());
	assert_eq!(resolution, Ok(f.join("node_modules/pck1/index.css")));
}

#[test]
fn should_fail_if_any_restriction_fails() {
	let f = super::fixture().join("restrictions");

	let resolver = Resolver::new(ResolveOptions {
		extensions: vec![".js".into(), ".css".into()],
		main_files: vec!["index".into()],
		restrictions: vec![
			Restriction::regex(r"\.css$").unwrap(),
			Restriction::regex(r"\.js$").unwrap(),
		],
		..ResolveOptions::default()
	});

	let resolution = resolver.resolve(&f, "pck1");
	assert_eq!(resolution, Err(ResolveError::NotFound("pck1".to_string())));
}
