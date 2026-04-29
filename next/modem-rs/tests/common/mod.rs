// tests/common/mod.rs

static LOG_INIT: std::sync::Once = std::sync::Once::new();

#[allow(dead_code)]
pub fn init_logger() {
    LOG_INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    });
}
