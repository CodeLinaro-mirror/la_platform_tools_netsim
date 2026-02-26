// tests/common/mod.rs

static LOG_INIT: std::sync::Once = std::sync::Once::new();

#[allow(dead_code)]
pub fn init_logger() {
    LOG_INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}
