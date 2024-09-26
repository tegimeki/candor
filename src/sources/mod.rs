pub mod peak_trace;

#[cfg(feature = "socketcan")]
pub mod socketcan;

pub trait Source {
    fn name(&self) -> String;
    fn baud(&self) -> u32;
}
