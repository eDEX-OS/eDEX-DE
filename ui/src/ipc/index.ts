import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  Settings,
  Shortcut,
  CpuInfo,
  RamInfo,
  NetInterface,
  ProcessEntry,
  SystemOverview,
  DiskInfo,
  BatteryInfo,
  FileEntry,
  FuzzyMatch,
  AppEntry,
  TailscaleStatus,
  TorStatus,
  VpnConnection,
} from '../types';

// Settings
export const loadSettings = () => invoke<Settings>('load_settings');
export const saveSettings = (settings: Settings) => invoke<void>('save_settings', { settings });
export const loadShortcuts = () => invoke<Shortcut[]>('load_shortcuts');
export const saveShortcuts = (shortcuts: Shortcut[]) => invoke<void>('save_shortcuts', { shortcuts });
export const getConfigDir = () => invoke<string>('get_config_dir');

// Terminal / PTY
export const spawnTerminal = (port: number, shell: string, cwd: string, env: [string, string][]) =>
  invoke<number>('spawn_terminal', { port, shell, cwd, env });
export const closeTerminal = (port: number) => invoke<void>('close_terminal', { port });
export const listTerminals = () => invoke<number[]>('list_terminals');

// Filesystem
export const listDir = (path: string, showDotfiles: boolean) =>
  invoke<FileEntry[]>('list_dir', { path, showDotfiles });
export const readFile = (path: string) => invoke<string>('read_file', { path });
export const renameEntry = (from: string, to: string) => invoke<void>('rename_entry', { from, to });
export const deleteEntry = (path: string) => invoke<void>('delete_entry', { path });
export const createDirectory = (path: string) => invoke<void>('create_directory', { path });
export const fuzzySearchFiles = (cwd: string, query: string) =>
  invoke<FuzzyMatch[]>('fuzzy_search_files', { cwd, query });

// Sysinfo
export const getCpuInfo = () => invoke<CpuInfo>('get_cpu_info');
export const getRamInfo = () => invoke<RamInfo>('get_ram_info');
export const getNetStats = () => invoke<NetInterface[]>('get_net_stats');
export const getProcessList = (excludeThreads: boolean) =>
  invoke<ProcessEntry[]>('get_process_list', { excludeThreads });
export const getSystemOverview = () => invoke<SystemOverview>('get_system_overview');
export const getDiskInfo = () => invoke<DiskInfo[]>('get_disk_info');
export const getBatteryInfo = () => invoke<BatteryInfo>('get_battery_info');

// Audio
export const playAudio = (path: string, volume: number) =>
  invoke<void>('play_audio', { path, volume });

// Update checker
export const checkForUpdate = (currentVersion: string) =>
  invoke<{ hasUpdate: boolean; latestVersion: string; currentVersion: string; releaseUrl: string }>(
    'check_for_update',
    { currentVersion },
  );

// Launcher
export const listApps = () => invoke<AppEntry[]>('list_apps');
export const searchApps = (query: string) =>
  invoke<{ app: AppEntry; score: number }[]>('search_apps', { query });
export const launchApp = (exec: string) => invoke<void>('launch_app', { exec });
export const getHyprlandLauncherBind = () => invoke<string>('get_hyprland_launcher_bind');
export const onToggleLauncher = (cb: () => void) =>
  listen<void>('toggle-launcher', () => cb());

// Hyprland
export interface WorkspaceInfo {
  id: number;
  name: string;
  monitor: string;
  windows: number;
  hasFullscreen: boolean;
  lastWindow: string;
}

export interface ActiveWindowInfo {
  address: string;
  class: string;
  title: string;
  workspaceId: number;
  workspaceName: string;
  monitor: string;
  pid: number;
  floating: boolean;
  width: number;
  height: number;
  at: [number, number];
}

export interface MonitorInfo {
  id: number;
  name: string;
  description: string;
  width: number;
  height: number;
  refreshRate: number;
  x: number;
  y: number;
  activeWorkspaceId: number;
  activeWorkspaceName: string;
  focused: boolean;
}

export interface HyprlandStatus {
  running: boolean;
  instance: string | null;
}

export const getHyprlandStatus = () => invoke<HyprlandStatus>('get_hyprland_status');
export const getWorkspaces = () => invoke<WorkspaceInfo[]>('get_workspaces');
export const getActiveWindow = () => invoke<ActiveWindowInfo | null>('get_active_window');
export const getMonitors = () => invoke<MonitorInfo[]>('get_monitors');
export const switchWorkspace = (id: number) => invoke<void>('switch_workspace', { id });
export const hyprDispatch = (action: string) => invoke<string>('hypr_dispatch', { action });
export const generateHyprlandConfig = () => invoke<string>('generate_hyprland_config');
export const saveHyprlandIntegrationConfig = () =>
  invoke<string>('save_hyprland_integration_config');
export const onHyprlandEvent = (cb: (event: { event: string; data: string }) => void) =>
  listen<{ event: string; data: string }>('hyprland-event', (e) => cb(e.payload));

// Audio / PipeWire
export interface AudioSink {
  index: number;
  name: string;
  description: string;
  volumePercent: number;
  muted: boolean;
  isDefault: boolean;
}
export const audioAvailable = () => invoke<boolean>('audio_available');
export const listAudioSinks = () => invoke<AudioSink[]>('list_audio_sinks');
export const getMasterVolume = () => invoke<number>('get_master_volume');
export const setMasterVolume = (percent: number) => invoke<void>('set_master_volume', { percent });
export const toggleMute = () => invoke<boolean>('toggle_mute');
export const setDefaultSink = (sinkName: string) => invoke<void>('set_default_sink', { sinkName });

// NetworkManager
export interface NetworkConnection {
  name: string;
  uuid: string;
  connType: string;
  device: string;
  active: boolean;
  state: string;
}
export interface WifiNetwork {
  ssid: string;
  bssid: string;
  signal: number;
  security: string;
  inUse: boolean;
  freq: string;
}
export const networkAvailable = () => invoke<boolean>('network_available');
export const listConnections = () => invoke<NetworkConnection[]>('list_connections');
export const wifiScan = () => invoke<WifiNetwork[]>('wifi_scan');
export const wifiConnect = (ssid: string, password?: string) =>
  invoke<void>('wifi_connect', { ssid, password: password ?? null });
export const nmDisconnect = (device: string) => invoke<void>('nm_disconnect', { device });
export const getActiveConnectionInfo = () => invoke<Record<string, string>>('get_active_connection_info');

// fprintd
export interface FprintdStatus {
  available: boolean;
  hasEnrolledFingers: boolean;
  deviceName?: string;
}
export interface VerifyResult {
  success: boolean;
  message: string;
}
export const getFprintdStatus = () => invoke<FprintdStatus>('fprintd_status');
export const fprintdVerify = () => invoke<VerifyResult>('fprintd_verify');
export const onFprintdStatus = (cb: (status: string) => void) =>
  listen<string>('fprintd-status', (e) => cb(e.payload));

// systemd
export interface SystemdUnit {
  name: string;
  loadState: string;
  activeState: string;
  subState: string;
  description: string;
  unitType: string;
  enabled?: boolean;
}
export const listUnits = (userUnits: boolean, unitTypeFilter?: string) =>
  invoke<SystemdUnit[]>('list_units', { userUnits, unitTypeFilter: unitTypeFilter ?? null });
export const unitAction = (unit: string, action: string, userUnits: boolean) =>
  invoke<string>('unit_action', { unit, action, userUnits });
export const getUnitLogs = (unit: string, lines?: number) =>
  invoke<string>('get_unit_logs', { unit, lines: lines ?? 50 });
export const getUnitStatus = (unit: string, userUnits: boolean) =>
  invoke<string>('get_unit_status', { unit, userUnits });

// Privacy — Tailscale
export const tailscaleAvailable = () => invoke<boolean>('tailscale_available');
export const tailscaleStatus = () => invoke<TailscaleStatus>('tailscale_status');
export const tailscaleLogin = () => invoke<string>('tailscale_login');
export const tailscaleLogout = () => invoke<void>('tailscale_logout');
export const tailscaleUp = (exitNode?: string) =>
  invoke<void>('tailscale_up', { exitNode: exitNode ?? null });
export const tailscaleDown = () => invoke<void>('tailscale_down');
export const tailscaleSetExitNode = (nodeIp?: string) =>
  invoke<void>('tailscale_set_exit_node', { nodeIp: nodeIp ?? null });

// Privacy — Tor
export const torAvailable = () => invoke<boolean>('tor_available');
export const torStatus = () => invoke<TorStatus>('tor_status');
export const torGetMode = () => invoke<string>('tor_get_mode');
export const torSetMode = (mode: string) => invoke<void>('tor_set_mode', { mode });
export const torRequestBridges = (bridgeType: string) =>
  invoke<string[]>('tor_request_bridges', { bridgeType });
export const torGetBridges = () => invoke<string[]>('tor_get_bridges');
export const torSetBridges = (bridges: string[]) => invoke<void>('tor_set_bridges', { bridges });

// Privacy — VPN
export const vpnListConnections = () => invoke<VpnConnection[]>('vpn_list_connections');
export const vpnConnect = (name: string) => invoke<void>('vpn_connect', { name });
export const vpnDisconnect = (name: string) => invoke<void>('vpn_disconnect', { name });
export const vpnImportWireguard = (configContent: string, profileName: string) =>
  invoke<void>('vpn_import_wireguard', { configContent, profileName });
