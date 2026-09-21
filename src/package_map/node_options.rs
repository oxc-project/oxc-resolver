use std::path::PathBuf;

/// Extracts the last `--experimental-package-map` path from `NODE_OPTIONS`.
///
/// Tokenization follows Node's `ParseNodeOptionsEnvVar` quoting and escaping rules.
pub(super) fn package_map_path_from_node_options(node_options: &str) -> Option<PathBuf> {
    let arguments = parse_node_options(node_options)?;
    let mut package_map_path = None;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        if let Some(path) = argument.strip_prefix("--experimental-package-map=") {
            package_map_path = (!path.is_empty()).then(|| PathBuf::from(path));
        } else if argument == "--experimental-package-map" {
            index += 1;
            package_map_path = arguments
                .get(index)
                .filter(|path| !path.is_empty() && !path.starts_with('-'))
                .map(PathBuf::from);
        }
        index += 1;
    }

    package_map_path
}

fn parse_node_options(node_options: &str) -> Option<Vec<String>> {
    let mut arguments = Vec::new();
    let mut chars = node_options.chars();
    let mut is_in_string = false;
    let mut will_start_new_argument = true;

    while let Some(mut character) = chars.next() {
        if character == '\\' && is_in_string {
            character = chars.next()?;
        } else if character == ' ' && !is_in_string {
            will_start_new_argument = true;
            continue;
        } else if character == '"' {
            is_in_string = !is_in_string;
            continue;
        }

        if will_start_new_argument {
            arguments.push(String::from(character));
            will_start_new_argument = false;
        } else {
            arguments.last_mut().expect("an argument has started").push(character);
        }
    }

    (!is_in_string).then_some(arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_does_not_require_cwd() {
        #[cfg(windows)]
        let path = r"C:\package-map.json";
        #[cfg(not(windows))]
        let path = "/package-map.json";

        assert_eq!(
            package_map_path_from_node_options(&format!(r#"--experimental-package-map="{path}""#)),
            Some(PathBuf::from(path)),
        );
    }
}
