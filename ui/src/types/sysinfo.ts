export interface CpuInfo {
  model: string;
  cores: number;
  usagePerCore: number[];
  totalUsage: number;
  frequencyMhz: number;
}

export interface RamInfo {
  total: number;
  used: number;
  available: number;
  swapTotal: number;
  swapUsed: number;
}

export interface NetInterface {
  name: string;
  rxBytes: number;
  txBytes: number;
  rxBytesPerSec: number;
  txBytesPerSec: number;
}

export interface ProcessEntry {
  pid: number;
  name: string;
  cpuUsage: number;
  memoryBytes: number;
}

export interface SystemOverview {
  osName: string;
  osVersion: string;
  hostname: string;
  uptimeSecs: number;
  bootTime: number;
}

export interface DiskInfo {
  name: string;
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
  fileSystem: string;
}

export interface BatteryInfo {
  hasBattery: boolean;
  percent?: number;
  isCharging?: boolean;
  acConnected?: boolean;
}
