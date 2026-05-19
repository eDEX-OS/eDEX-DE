use std::collections::VecDeque;

use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

use crate::battery::read_battery;

#[derive(Clone, Debug, Default)]
pub struct SysSnapshot {
    pub cpu_usage: Vec<f32>,
    pub cpu_freq_mhz: Vec<u64>,
    pub cpu_model: String,
    pub cpu_total: f32,
    pub ram_used_kb: u64,
    pub ram_total_kb: u64,
    pub swap_used_kb: u64,
    pub swap_total_kb: u64,
    pub net_tx_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_kbps: f32,
    pub net_rx_kbps: f32,
    pub net_tx_history: Vec<f32>,
    pub net_rx_history: Vec<f32>,
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
    pub status: String,
}

pub struct SysmonCollector {
    system: System,
    networks: Networks,
    disks: Disks,
    prev_tx: u64,
    prev_rx: u64,
    tx_history: VecDeque<f32>,
    rx_history: VecDeque<f32>,
    history_len: usize,
    snapshot: SysSnapshot,
}

impl SysmonCollector {
    pub fn new() -> Self {
        let mut collector = Self {
            system: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            prev_tx: 0,
            prev_rx: 0,
            tx_history: VecDeque::new(),
            rx_history: VecDeque::new(),
            history_len: 60,
            snapshot: SysSnapshot::default(),
        };
        collector.refresh();
        collector
    }

    pub fn refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.networks.refresh(true);
        self.disks.refresh(true);

        let cpu_usage = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect::<Vec<_>>();
        let cpu_freq_mhz = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.frequency())
            .collect::<Vec<_>>();
        let cpu_total = if cpu_usage.is_empty() {
            0.0
        } else {
            cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
        };
        let cpu_model = self
            .system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let net_tx_bytes = self
            .networks
            .values()
            .map(|network| network.transmitted())
            .sum::<u64>();
        let net_rx_bytes = self
            .networks
            .values()
            .map(|network| network.received())
            .sum::<u64>();
        let tx_delta = net_tx_bytes.saturating_sub(self.prev_tx);
        let rx_delta = net_rx_bytes.saturating_sub(self.prev_rx);
        self.prev_tx = net_tx_bytes;
        self.prev_rx = net_rx_bytes;

        let net_tx_kbps = tx_delta as f32 / 1024.0;
        let net_rx_kbps = rx_delta as f32 / 1024.0;
        push_history(&mut self.tx_history, net_tx_kbps, self.history_len);
        push_history(&mut self.rx_history, net_rx_kbps, self.history_len);

        let disks = self
            .disks
            .list()
            .iter()
            .map(|disk| {
                let total_bytes = disk.total_space();
                let available_bytes = disk.available_space();
                DiskInfo {
                    mount: disk.mount_point().display().to_string(),
                    used_bytes: total_bytes.saturating_sub(available_bytes),
                    total_bytes,
                    fs_type: disk.file_system().to_string_lossy().to_string(),
                }
            })
            .collect::<Vec<_>>();

        let mut processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_pct: process.cpu_usage(),
                mem_kb: process.memory() / 1024,
                status: format!("{:?}", process.status()),
            })
            .collect::<Vec<_>>();
        processes.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.mem_kb.cmp(&a.mem_kb))
        });
        processes.truncate(8);

        let battery = read_battery();
        let battery_pct = battery.map(|(pct, _)| pct);
        let battery_charging = battery.map(|(_, charging)| charging).unwrap_or(false);

        self.snapshot = SysSnapshot {
            cpu_usage,
            cpu_freq_mhz,
            cpu_model,
            cpu_total,
            ram_used_kb: self.system.used_memory() / 1024,
            ram_total_kb: self.system.total_memory() / 1024,
            swap_used_kb: self.system.used_swap() / 1024,
            swap_total_kb: self.system.total_swap() / 1024,
            net_tx_bytes,
            net_rx_bytes,
            net_tx_kbps,
            net_rx_kbps,
            net_tx_history: self.tx_history.iter().copied().collect(),
            net_rx_history: self.rx_history.iter().copied().collect(),
            disks,
            processes,
            battery_pct,
            battery_charging,
        };
    }

    pub fn snapshot(&self) -> SysSnapshot {
        self.snapshot.clone()
    }
}

impl Default for SysmonCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn push_history(history: &mut VecDeque<f32>, value: f32, max_len: usize) {
    history.push_back(value);
    while history.len() > max_len {
        history.pop_front();
    }
}
