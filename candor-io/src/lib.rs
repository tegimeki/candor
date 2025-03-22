pub mod trc;

#[cfg(feature = "socketcan")]
pub mod socketcan;

pub trait Source {
    fn name(&self) -> String;
    fn baud(&self) -> u32;
}
