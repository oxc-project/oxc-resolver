use std::path::PathBuf;

use indexmap::IndexMap;
use napi_derive::napi;
use oxc_resolver::{CompilerOptions, TsConfig};

fn pathbufs_into_strings(paths: Option<Vec<PathBuf>>) -> Option<Vec<String>> {
    paths.map(|paths| paths.into_iter().map(|path| path.to_string_lossy().into_owned()).collect())
}

#[napi(object, object_from_js = false)]
pub struct TsconfigResult {
    pub tsconfig_paths: Vec<String>,
    pub tsconfig: Tsconfig,
}

impl From<&TsConfig> for TsconfigResult {
    fn from(tsconfig: &TsConfig) -> Self {
        Self {
            tsconfig_paths: vec![tsconfig.path.to_string_lossy().into_owned()],
            tsconfig: tsconfig.into(),
        }
    }
}

#[napi(object, object_from_js = false)]
pub struct Tsconfig {
    pub files: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub compiler_options: TsconfigCompilerOptions,
}

impl From<&TsConfig> for Tsconfig {
    fn from(tsconfig: &TsConfig) -> Self {
        Self {
            files: pathbufs_into_strings(tsconfig.files.clone()),
            include: pathbufs_into_strings(tsconfig.include.clone()),
            exclude: pathbufs_into_strings(tsconfig.exclude.clone()),
            compiler_options: (&tsconfig.compiler_options).into(),
        }
    }
}

#[napi(object, object_from_js = false)]
pub struct TsconfigCompilerOptions {
    pub base_url: Option<String>,
    pub paths: Option<IndexMap<String, Vec<String>>>,
    pub experimental_decorators: Option<bool>,
    pub emit_decorator_metadata: Option<bool>,
    pub strict: Option<bool>,
    pub strict_null_checks: Option<bool>,
    pub use_define_for_class_fields: Option<bool>,
    pub rewrite_relative_import_extensions: Option<bool>,
    pub jsx: Option<String>,
    pub jsx_factory: Option<String>,
    pub jsx_fragment_factory: Option<String>,
    pub jsx_import_source: Option<String>,
    pub verbatim_module_syntax: Option<bool>,
    pub preserve_value_imports: Option<bool>,
    pub imports_not_used_as_values: Option<String>,
    pub target: Option<String>,
    pub module: Option<String>,
    pub allow_js: Option<bool>,
    pub root_dirs: Option<Vec<String>>,
    pub out_dir: Option<String>,
    pub declaration_dir: Option<String>,
    pub resolve_json_module: Option<bool>,
    pub check_js: Option<bool>,
}

impl From<&CompilerOptions> for TsconfigCompilerOptions {
    fn from(options: &CompilerOptions) -> Self {
        Self {
            base_url: options.base_url.as_ref().map(|path| path.to_string_lossy().into_owned()),
            paths: options.paths.as_ref().map(|paths| {
                paths
                    .iter()
                    .map(|(key, paths)| {
                        (
                            key.clone(),
                            pathbufs_into_strings(Some(paths.clone())).unwrap_or_default(),
                        )
                    })
                    .collect()
            }),
            experimental_decorators: options.experimental_decorators,
            emit_decorator_metadata: options.emit_decorator_metadata,
            strict: options.strict,
            strict_null_checks: options.strict_null_checks,
            use_define_for_class_fields: options.use_define_for_class_fields,
            rewrite_relative_import_extensions: options.rewrite_relative_import_extensions,
            jsx: options.jsx.clone(),
            jsx_factory: options.jsx_factory.clone(),
            jsx_fragment_factory: options.jsx_fragment_factory.clone(),
            jsx_import_source: options.jsx_import_source.clone(),
            verbatim_module_syntax: options.verbatim_module_syntax,
            preserve_value_imports: options.preserve_value_imports,
            imports_not_used_as_values: options.imports_not_used_as_values.clone(),
            target: options.target.clone(),
            module: options.module.clone(),
            allow_js: options.allow_js,
            root_dirs: pathbufs_into_strings(options.root_dirs.clone()),
            out_dir: options.out_dir.as_ref().map(|path| path.to_string_lossy().into_owned()),
            declaration_dir: options
                .declaration_dir
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            resolve_json_module: options.resolve_json_module,
            check_js: options.check_js,
        }
    }
}
