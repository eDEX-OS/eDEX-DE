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
