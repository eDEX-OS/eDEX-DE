import { useState } from 'preact/hooks';
import { CpuPanel } from './CpuPanel';
import { RamPanel } from './RamPanel';
import { NetPanel } from './NetPanel';
import { ProcessList } from './ProcessList';
import { DiskPanel } from './DiskPanel';

type Tab = 'cpu' | 'mem' | 'net' | 'proc' | 'disk';

export function SysinfoSidebar() {
  const [tab, setTab] = useState<Tab>('cpu');

  const tabs: { id: Tab; label: string }[] = [
    { id: 'cpu', label: 'CPU' },
    { id: 'mem', label: 'MEM' },
    { id: 'net', label: 'NET' },
    { id: 'proc', label: 'PROC' },
    { id: 'disk', label: 'DISK' },
  ];

  return (
    <div class="sysinfo-sidebar">
      <div class="sysinfo-tabs">
        {tabs.map((t) => (
          <button
            key={t.id}
            class={`sysinfo-tab ${tab === t.id ? 'active' : ''}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div class="sysinfo-content">
        {tab === 'cpu' && <CpuPanel />}
        {tab === 'mem' && <RamPanel />}
        {tab === 'net' && <NetPanel />}
        {tab === 'proc' && <ProcessList />}
        {tab === 'disk' && <DiskPanel />}
      </div>
    </div>
  );
}
