#![no_main]

use libfuzzer_sys::fuzz_target;
use unrs_resolver::Resolver;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if s.chars().all(|s| !s.is_control()) {
            let resolver = Resolver::default();
            let cwd = std::env::current_dir().unwrap();
            let _ = resolver.resolve(cwd, &s);
        }
    }
});
