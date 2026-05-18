import { createContext } from 'preact';
import type { ComponentChildren } from 'preact';
import { useContext, useEffect, useState } from 'preact/hooks';
import type {
  CpuInfo,
  RamInfo,
  NetInterface,
  ProcessEntry,
  SystemOverview,
  DiskInfo,
  BatteryInfo,
} from '../types';
import {
  getCpuInfo,
  getRamInfo,
  getNetStats,
  getProcessList,
  getSystemOverview,
  getDiskInfo,
  getBatteryInfo,
} from '../ipc';

interface SysinfoState {
  cpu: CpuInfo | null;
  ram: RamInfo | null;
  net: NetInterface[];
  processes: ProcessEntry[];
  overview: SystemOverview | null;
  disks: DiskInfo[];
  battery: BatteryInfo | null;
}

interface SysinfoContextValue extends SysinfoState {
  refresh: () => void;
}

const SysinfoContext = createContext<SysinfoContextValue>({
  cpu: null,
  ram: null,
  net: [],
  processes: [],
  overview: null,
  disks: [],
  battery: null,
  refresh: () => {},
});

export function SysinfoProvider({
  children,
  intervalMs = 2000,
}: {
  children: ComponentChildren;
  intervalMs?: number;
}) {
  const [state, setState] = useState<SysinfoState>({
    cpu: null,
    ram: null,
    net: [],
    processes: [],
    overview: null,
    disks: [],
    battery: null,
  });

  const fetchAll = async () => {
    const [cpu, ram, net, processes, overview, disks, battery] = await Promise.allSettled([
      getCpuInfo(),
      getRamInfo(),
      getNetStats(),
      getProcessList(true),
      getSystemOverview(),
      getDiskInfo(),
      getBatteryInfo(),
    ]);

    setState({
      cpu: cpu.status === 'fulfilled' ? cpu.value : null,
      ram: ram.status === 'fulfilled' ? ram.value : null,
      net: net.status === 'fulfilled' ? net.value : [],
      processes: processes.status === 'fulfilled' ? processes.value : [],
      overview: overview.status === 'fulfilled' ? overview.value : null,
      disks: disks.status === 'fulfilled' ? disks.value : [],
      battery: battery.status === 'fulfilled' ? battery.value : null,
    });
  };

  useEffect(() => {
    void fetchAll();
    const id = window.setInterval(() => {
      void fetchAll();
    }, intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);

  return (
    <SysinfoContext.Provider
      value={{
        ...state,
        refresh: () => {
          void fetchAll();
        },
      }}
    >
      {children}
    </SysinfoContext.Provider>
  );
}

export const useSysinfo = () => useContext(SysinfoContext);
