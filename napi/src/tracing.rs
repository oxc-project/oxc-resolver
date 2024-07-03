use std::sync::OnceLock;

use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// To debug `oxc_resolver`:
/// `OXC_LOG=DEBUG your program`
pub fn init_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        // Usage without the `regex` feature.
        // <https://github.com/tokio-rs/tracing/issues/1436#issuecomment-918528013>
        tracing_subscriber::registry()
            .with(std::env::var("OXC_LOG").map_or_else(
                |_| Targets::new(),
                |env_var| {
                    use std::str::FromStr;
                    Targets::from_str(&env_var).unwrap()
                },
            ))
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}
