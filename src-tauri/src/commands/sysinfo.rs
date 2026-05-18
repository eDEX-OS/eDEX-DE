use serde::Serialize;
use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub model: String,
    pub cores: usize,
    pub usage_per_core: Vec<f32>,
    pub total_usage: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetInterface {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemOverview {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime_secs: u64,
    pub boot_time: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub file_system: String,
}

#[tauri::command]
pub fn get_cpu_info() -> Result<CpuInfo, String> {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_all();

    let cpus = sys.cpus();
    let usage_per_core: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
    let total_usage = usage_per_core.iter().sum::<f32>() / usage_per_core.len().max(1) as f32;
    let model = cpus
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_default();
    let frequency_mhz = cpus.first().map(|cpu| cpu.frequency()).unwrap_or(0);

    Ok(CpuInfo {
        model,
        cores: cpus.len(),
        usage_per_core,
        total_usage,
        frequency_mhz,
    })
}

#[tauri::command]
pub fn get_ram_info() -> Result<RamInfo, String> {
    let mut sys = System::new();
    sys.refresh_memory();
    Ok(RamInfo {
        total: sys.total_memory(),
        used: sys.used_memory(),
        available: sys.available_memory(),
        swap_total: sys.total_swap(),
        swap_used: sys.used_swap(),
    })
}

#[tauri::command]
pub fn get_net_stats() -> Result<Vec<NetInterface>, String> {
    let mut nets = Networks::new_with_refreshed_list();
    std::thread::sleep(std::time::Duration::from_millis(500));
    nets.refresh();

    Ok(nets
        .iter()
        .map(|(name, data)| NetInterface {
            name: name.clone(),
            rx_bytes: data.total_received(),
            tx_bytes: data.total_transmitted(),
            rx_bytes_per_sec: data.received() as f64 * 2.0,
            tx_bytes_per_sec: data.transmitted() as f64 * 2.0,
        })
        .collect())
}

#[tauri::command]
pub fn get_process_list(exclude_threads: bool) -> Result<Vec<ProcessEntry>, String> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut procs: Vec<ProcessEntry> = sys
        .processes()
        .values()
        .filter_map(|process| {
            if exclude_threads && process.thread_kind().is_some() {
                return None;
            }

            Some(ProcessEntry {
                pid: process.pid().as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_bytes: process.memory(),
            })
        })
        .collect();

    procs.sort_by(|a, b| {
        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    procs.truncate(50);
    Ok(procs)
}

#[tauri::command]
pub fn get_system_overview() -> Result<SystemOverview, String> {
    Ok(SystemOverview {
        os_name: System::name().unwrap_or_else(|| "Linux".into()),
        os_version: System::os_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
        uptime_secs: System::uptime(),
        boot_time: System::boot_time(),
    })
}

#[tauri::command]
pub fn get_disk_info() -> Result<Vec<DiskInfo>, String> {
    let disks = Disks::new_with_refreshed_list();
    Ok(disks
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            file_system: disk.file_system().to_string_lossy().to_string(),
        })
        .collect())
}

#[tauri::command]
pub fn get_battery_info() -> Result<serde_json::Value, String> {
    let path = std::path::Path::new("/sys/class/power_supply");
    if !path.exists() {
        return Ok(serde_json::json!({ "hasBattery": false }));
    }

    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let battery_type_path = entry.path().join("type");
        if let Ok(battery_type) = std::fs::read_to_string(&battery_type_path) {
            if battery_type.trim() == "Battery" {
                let capacity = std::fs::read_to_string(entry.path().join("capacity"))
                    .ok()
                    .and_then(|value| value.trim().parse::<u32>().ok())
                    .unwrap_or(0);
                let status = std::fs::read_to_string(entry.path().join("status")).unwrap_or_default();
                let is_charging = status.trim() == "Charging";
                let ac_connected = status.trim() == "Full" || is_charging;

                return Ok(serde_json::json!({
                    "hasBattery": true,
                    "percent": capacity,
                    "isCharging": is_charging,
                    "acConnected": ac_connected
                }));
            }
        }
    }

    Ok(serde_json::json!({ "hasBattery": false }))
}
