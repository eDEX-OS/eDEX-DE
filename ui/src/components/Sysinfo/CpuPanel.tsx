import { useSysinfo } from '../../context';
import { formatPercent } from '../../utils';

export function CpuPanel() {
  const { cpu } = useSysinfo();

  if (!cpu) return <div class="sysinfo-loading">CPU</div>;

  return (
    <div class="sysinfo-panel cpu-panel">
      <div class="sysinfo-header">CPU</div>
      <div class="cpu-model">{cpu.model}</div>
      <div class="cpu-cores-label">
        {cpu.cores} cores @ {cpu.frequencyMhz} MHz
      </div>
      <div class="cpu-total">
        <div class="gauge-label">Total</div>
        <div class="gauge-bar">
          <div class="gauge-fill" style={{ width: `${cpu.totalUsage}%` }} />
        </div>
        <div class="gauge-value">{formatPercent(cpu.totalUsage)}</div>
      </div>
      <div class="cpu-cores">
        {cpu.usagePerCore.map((usage, i) => (
          <div key={i} class="core-bar" title={`Core ${i}: ${formatPercent(usage)}`}>
            <div class="core-fill" style={{ height: `${usage}%` }} />
          </div>
        ))}
      </div>
    </div>
  );
}
