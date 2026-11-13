use std::path::PathBuf;

use napi::{
    Env,
    bindgen_prelude::{Function, JsObjectValue, Object},
};
use oxc_resolver::ResolveOptions;

pub fn with_exec_argv(env: Env, mut options: ResolveOptions) -> napi::Result<ResolveOptions> {
    let Some(process) = env.get_global()?.get_named_property::<Option<Object>>("process")? else {
        return Ok(options);
    };
    let arguments =
        process.get_named_property::<Option<Vec<String>>>("execArgv")?.unwrap_or_default();
    // Node has already separated runtime options from the script and its arguments.
    // Scan backwards because the last command-line value overrides earlier values and NODE_OPTIONS.
    for (index, argument) in arguments.iter().enumerate().rev() {
        let path = if let Some(path) = argument.strip_prefix("--experimental-package-map=") {
            Some(path)
        } else if argument == "--experimental-package-map" {
            arguments.get(index + 1).map(String::as_str)
        } else {
            continue;
        };
        let path = path.filter(|path| !path.is_empty()).ok_or_else(|| {
            napi::Error::from_reason("--experimental-package-map requires a path")
        })?;
        let mut path = PathBuf::from(path);
        if path.is_relative() {
            // WASI's working directory may differ from the Node.js host's.
            let cwd = process.get_named_property::<Function<(), String>>("cwd")?;
            path = PathBuf::from(cwd.call(())?).join(path);
        }
        options.package_map = Some(path);
        break;
    }
    Ok(options)
}
