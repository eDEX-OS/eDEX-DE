import { useState } from 'preact/hooks';
import { CpuPanel } from './CpuPanel';
import { RamPanel } from './RamPanel';
import { NetPanel } from './NetPanel';
import { ProcessList } from './ProcessList';
import { DiskPanel } from './DiskPanel';
import { NetworkPanel } from '../Network';
import { ServiceManager } from '../SystemServices';

type Tab = 'cpu' | 'mem' | 'net' | 'proc' | 'disk' | 'nm' | 'svc';

export function SysinfoSidebar() {
  const [tab, setTab] = useState<Tab>('cpu');

  const tabs: { id: Tab; label: string }[] = [
    { id: 'cpu', label: 'CPU' },
    { id: 'mem', label: 'MEM' },
    { id: 'net', label: 'NET' },
    { id: 'proc', label: 'PROC' },
    { id: 'disk', label: 'DISK' },
    { id: 'nm', label: 'WIFI' },
    { id: 'svc', label: 'SVC' },
  ];

  return (
    <div class="sysinfo-sidebar">
      <div class="sysinfo-tabs" style={{ flexWrap: 'wrap' }}>
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
        {tab === 'nm' && <NetworkPanel />}
        {tab === 'svc' && <ServiceManager />}
      </div>
    </div>
  );
}
