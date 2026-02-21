use std::time::Duration;

pub const LABEL_WIDTH: usize = 20;

#[derive(Debug, Clone)]
pub struct ClientParams {
    pub payload_size: usize,
    pub proto: String,
    pub target: String,
    pub expect_eof: bool,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Throughput {
    pub bytes: usize,
    pub duration: Duration,
}
