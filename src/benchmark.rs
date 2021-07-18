pub use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Benchmark<T> {
    pub content: T,
    pub elapsed: Duration,
}
