//! sysinfo-backed collector; call `refresh` from a 1 s timer.

use std::{collections::VecDeque, time::Instant};

use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

use crate::{
    battery::read_battery,
    gpu::{GpuCollector, GpuInfo},
};

#[derive(Clone, Debug, Default)]
pub struct SysSnapshot {
    pub cpu_usage: Vec<f32>,
    pub cpu_freq_mhz: u64,
    pub cpu_model: String,
    pub cpu_total: f32,
    pub cpu_temp_c: Option<f32>,
    pub load_avg: [f32; 3],
    pub uptime_secs: u64,
    pub kernel: String,
    pub hostname: String,
    pub ram_used_kb: u64,
    pub ram_total_kb: u64,
    /// Reclaimable (page cache, buffers): neither used nor free.
    pub ram_cached_kb: u64,
    pub gpus: Vec<GpuInfo>,
    /// Every temperature sensor: (label, °C), hottest first.
    pub temps: Vec<(String, f32)>,
    pub swap_used_kb: u64,
    pub swap_total_kb: u64,
    pub net_tx_kbps: f32,
    pub net_rx_kbps: f32,
    pub net_tx_history: Vec<f32>,
    pub net_rx_history: Vec<f32>,
    pub net_iface: String,
    pub net_ip: String,
    pub disks: Vec<DiskInfo>,
    pub processes: Vec<ProcInfo>,
    pub battery_pct: Option<u8>,
    pub battery_charging: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DiskInfo {
    pub mount: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub fs_type: String,
}

#[derive(Clone, Debug, Default)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_kb: u64,
}

pub struct SysmonCollector {
    system: System,
    networks: Networks,
    disks: Disks,
    components: Components,
    last_refresh: Instant,
    tx_history: VecDeque<f32>,
    rx_history: VecDeque<f32>,
    history_len: usize,
    snapshot: SysSnapshot,
    ticks: u64,
    gpu: GpuCollector,
    gpus: Vec<GpuInfo>,
}

impl SysmonCollector {
    pub fn new() -> Self {
        let mut collector = Self {
            system: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_refresh: Instant::now(),
            tx_history: VecDeque::new(),
            rx_history: VecDeque::new(),
            history_len: 60,
            snapshot: SysSnapshot::default(),
            ticks: 0,
            gpu: GpuCollector::new(),
            gpus: Vec::new(),
        };
        collector.refresh();
        collector
    }

    /// Refresh everything that changes quickly. Slow lists (disks, components) refresh
    /// every 10th call.
    pub fn refresh(&mut self) {
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_refresh)
            .as_secs_f32()
            .max(0.05);
        self.last_refresh = now;
        self.ticks += 1;

        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.networks.refresh(true);
        if self.ticks % 10 == 1 {
            self.disks.refresh(true);
        }
        if self.ticks % 2 == 1 {
            self.components.refresh(true);
            // nvidia-smi is a process spawn: every other second is plenty.
            self.gpus = self.gpu.sample();
        }

        let cpu_usage: Vec<f32> = self.system.cpus().iter().map(|c| c.cpu_usage()).collect();
        let cpu_total = if cpu_usage.is_empty() {
            0.0
        } else {
            cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
        };
        let cpu_freq_mhz = self
            .system
            .cpus()
            .iter()
            .map(|c| c.frequency())
            .max()
            .unwrap_or(0);
        let cpu_model = self
            .system
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".into());
        let cpu_temp_c = self
            .components
            .iter()
            .filter(|c| {
                let l = c.label().to_ascii_lowercase();
                l.contains("package")
                    || l.contains("tctl")
                    || l.contains("cpu")
                    || l.contains("core 0")
            })
            .filter_map(|c| c.temperature())
            .fold(None, |acc: Option<f32>, t| {
                Some(acc.map_or(t, |a| a.max(t)))
            });

        // sysinfo reports bytes since the previous refresh; convert to kB/s.
        let tx_bytes: u64 = self.networks.values().map(|n| n.transmitted()).sum();
        let rx_bytes: u64 = self.networks.values().map(|n| n.received()).sum();
        let net_tx_kbps = tx_bytes as f32 / 1024.0 / elapsed;
        let net_rx_kbps = rx_bytes as f32 / 1024.0 / elapsed;
        push_history(&mut self.tx_history, net_tx_kbps, self.history_len);
        push_history(&mut self.rx_history, net_rx_kbps, self.history_len);

        let (net_iface, net_ip) = primary_interface(&self.networks);

        let mut disks: Vec<DiskInfo> = self
            .disks
            .list()
            .iter()
            .filter(|d| {
                let mp = d.mount_point().to_string_lossy();
                !(mp.starts_with("/boot") && mp.len() > 5)
                    && !mp.starts_with("/run")
                    && !mp.starts_with("/snap")
                    && d.total_space() > 0
            })
            .map(|d| DiskInfo {
                mount: d.mount_point().display().to_string(),
                used_bytes: d.total_space().saturating_sub(d.available_space()),
                total_bytes: d.total_space(),
                fs_type: d.file_system().to_string_lossy().to_string(),
            })
            .collect();
        disks.sort_by(|a, b| {
            a.mount
                .len()
                .cmp(&b.mount.len())
                .then_with(|| a.mount.cmp(&b.mount))
        });
        disks.dedup_by(|a, b| {
            a.total_bytes == b.total_bytes && a.used_bytes == b.used_bytes && a.fs_type == b.fs_type
        });

        let mut processes: Vec<ProcInfo> = self
            .system
            .processes()
            .iter()
            .filter(|(_, p)| p.thread_kind().is_none())
            .map(|(pid, p)| ProcInfo {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu_pct: p.cpu_usage(),
                mem_kb: p.memory() / 1024,
            })
            .collect();
        processes.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.mem_kb.cmp(&a.mem_kb))
        });
        processes.truncate(12);

        let mut temps: Vec<(String, f32)> = self
            .components
            .iter()
            .filter_map(|c| Some((short_sensor_label(c.label()), c.temperature()?)))
            .filter(|(_, t)| *t > 0.0 && *t < 150.0)
            .collect();
        temps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        temps.dedup_by(|a, b| a.0 == b.0);

        let battery = read_battery();
        let load = System::load_average();
        self.snapshot = SysSnapshot {
            cpu_usage,
            cpu_freq_mhz,
            cpu_model,
            cpu_total,
            cpu_temp_c,
            load_avg: [load.one as f32, load.five as f32, load.fifteen as f32],
            uptime_secs: System::uptime(),
            kernel: format!(
                "{} {}",
                System::name().unwrap_or_else(|| "Linux".into()),
                System::kernel_version().unwrap_or_default()
            ),
            hostname: System::host_name().unwrap_or_else(|| "edex".into()),
            ram_used_kb: self.system.used_memory() / 1024,
            ram_total_kb: self.system.total_memory() / 1024,
            ram_cached_kb: self
                .system
                .available_memory()
                .saturating_sub(self.system.free_memory())
                / 1024,
            gpus: self.gpus.clone(),
            temps,
            swap_used_kb: self.system.used_swap() / 1024,
            swap_total_kb: self.system.total_swap() / 1024,
            net_tx_kbps,
            net_rx_kbps,
            net_tx_history: self.tx_history.iter().copied().collect(),
            net_rx_history: self.rx_history.iter().copied().collect(),
            net_iface,
            net_ip,
            disks,
            processes,
            battery_pct: battery.map(|(p, _)| p),
            battery_charging: battery.map(|(_, c)| c).unwrap_or(false),
        };
    }

    pub fn snapshot(&self) -> &SysSnapshot {
        &self.snapshot
    }

    /// Whether a process with the given name is running (used by privacy probes).
    pub fn process_running(&self, name: &str) -> bool {
        self.system
            .processes()
            .values()
            .any(|p| p.name().to_string_lossy() == name)
    }
}

impl Default for SysmonCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// "amdgpu edge" → "GPU edge", "k10temp Tctl" → "CPU Tctl", "nvme Composite" → "NVMe".
fn short_sensor_label(label: &str) -> String {
    let mut words = label.split_whitespace();
    let chip = words.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<&str> = words.collect();
    let kind = match chip.as_str() {
        "k10temp" | "coretemp" | "zenpower" | "cpu_thermal" | "x86_pkg_temp" => "CPU",
        "amdgpu" | "nouveau" | "radeon" | "i915" | "xe" => "GPU",
        c if c.starts_with("nvme") => "NVMe",
        "acpitz" => "ACPI",
        "iwlwifi_1" | "iwlwifi" | "mt7921_phy0" => "WiFi",
        _ => "",
    };
    let rest = rest.join(" ");
    let rest = rest.trim_start_matches("Package id 0").trim();
    match (kind, rest) {
        ("", _) => label.to_string(),
        ("NVMe", "Composite") | (_, "") => kind.to_string(),
        (k, r) => format!("{k} {r}"),
    }
}

fn push_history(history: &mut VecDeque<f32>, value: f32, max_len: usize) {
    history.push_back(value);
    while history.len() > max_len {
        history.pop_front();
    }
}

/// Interface carrying the default route and its first IPv4 address.
fn primary_interface(networks: &Networks) -> (String, String) {
    let iface = default_route_interface().unwrap_or_default();
    if iface.is_empty() {
        return (String::new(), String::new());
    }
    let ip = networks
        .get(&iface)
        .and_then(|n| {
            n.ip_networks()
                .iter()
                .find(|ip| ip.addr.is_ipv4())
                .map(|ip| ip.addr.to_string())
        })
        .unwrap_or_default();
    (iface, ip)
}

fn default_route_interface() -> Option<String> {
    let route = std::fs::read_to_string("/proc/net/route").ok()?;
    route
        .lines()
        .skip(1)
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            (cols.len() > 3
                && cols[1] == "00000000"
                && u32::from_str_radix(cols[3], 16)
                    .map(|f| f & 2 != 0)
                    .unwrap_or(false))
            .then(|| cols[0].to_string())
        })
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_has_basic_fields() {
        let c = SysmonCollector::new();
        let s = c.snapshot();
        assert!(!s.cpu_usage.is_empty());
        assert!(s.ram_total_kb > 0);
        assert!(!s.kernel.is_empty());
        assert_eq!(s.net_tx_history.len(), 1);
    }
}
