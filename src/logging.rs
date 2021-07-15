use std::{
    env,
    ffi::OsStr,
    sync::Once,
};

mod env_keys {
    pub const RUST_LOG: &'static str = "RUST_LOG";
}

fn env_or_default<K, V, F>(key: K, value: F)
where
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
    F: FnOnce() -> V,
{
    let key = key.as_ref();
    if env::var_os(key).is_none() {
        env::set_var(key, value());
    }
}

#[cfg(debug_assertions)]
pub const DEFAULT_LOG_LEVEL: &'static str = "trace";
#[cfg(not(debug_assertions))]
pub const DEFAULT_LOG_LEVEL: &'static str = "info";

static INIT_LOGGING: Once = Once::new();

pub fn init() {
    INIT_LOGGING.call_once(|| {
        env_or_default(env_keys::RUST_LOG, || format!("{}={}", env!("CARGO_CRATE_NAME"), DEFAULT_LOG_LEVEL));
        env_logger::init();
    });
}
