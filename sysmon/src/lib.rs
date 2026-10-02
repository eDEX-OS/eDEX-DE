//! System statistics and privacy probes for the dashboard.

pub mod battery;
pub mod collector;
pub mod gpu;
pub mod privacy;

pub use collector::{DiskInfo, ProcInfo, SysSnapshot, SysmonCollector};
pub use gpu::{GpuCollector, GpuInfo};
pub use privacy::{PrivacyProbe, PrivacyStatus};
