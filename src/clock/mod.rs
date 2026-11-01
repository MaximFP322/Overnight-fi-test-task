mod system;

pub use system::SystemClock;

pub trait Clock {
    fn now_ns(&self) -> u64;
}
