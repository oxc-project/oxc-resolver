#[must_use]
pub fn mangle_scoped_package_name(package_name: &str) -> String {
    if let Some(stripped) = package_name.strip_prefix('@') {
        if let Some((scope, name)) = stripped.split_once('/') {
            return format!("{scope}__{name}");
        }
    }
    package_name.to_string()
}

#[must_use]
pub fn get_types_package_name(package_name: &str) -> String {
    let mangled = mangle_scoped_package_name(package_name);
    format!("@types/{mangled}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangle_scoped_package_name() {
        assert_eq!(mangle_scoped_package_name("@foo/bar"), "foo__bar");
        assert_eq!(mangle_scoped_package_name("@angular/core"), "angular__core");
        assert_eq!(mangle_scoped_package_name("react"), "react");
        assert_eq!(mangle_scoped_package_name("lodash"), "lodash");
    }

    #[test]
    fn test_resolve_at_types_package() {
        assert_eq!(get_types_package_name("@foo/bar"), "@types/foo__bar");
        assert_eq!(get_types_package_name("@angular/core"), "@types/angular__core");
        assert_eq!(get_types_package_name("react"), "@types/react");
        assert_eq!(get_types_package_name("lodash"), "@types/lodash");
    }
}
