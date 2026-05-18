import { useSysinfo } from '../context';
import { formatPercent } from '../utils';

export function StatusBar() {
  const { cpu, ram, battery } = useSysinfo();
  const ramPercent = ram && ram.total > 0 ? formatPercent((ram.used / ram.total) * 100) : '--';

  return (
    <div class="statusbar">
      <span class="status-item">
        CPU: <strong>{cpu ? formatPercent(cpu.totalUsage) : '--'}</strong>
      </span>
      <span class="status-item">
        RAM: <strong>{ramPercent}</strong>
      </span>
      {battery?.hasBattery && (
        <span class="status-item">
          BAT: <strong>{battery.percent ?? '--'}%{battery.isCharging ? ' ⚡' : ''}</strong>
        </span>
      )}
    </div>
  );
}
