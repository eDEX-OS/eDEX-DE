import { useSysinfo } from '../../context';
import { formatBytes, formatPercent } from '../../utils';

export function ProcessList() {
  const { processes } = useSysinfo();

  return (
    <div class="sysinfo-panel process-panel">
      <div class="sysinfo-header">PROCESSES</div>
      <div class="process-list">
        <div class="process-header">
          <span class="proc-pid">PID</span>
          <span class="proc-name">NAME</span>
          <span class="proc-cpu">CPU%</span>
          <span class="proc-mem">MEM</span>
        </div>
        {processes.slice(0, 20).map((proc) => (
          <div key={proc.pid} class="process-row">
            <span class="proc-pid">{proc.pid}</span>
            <span class="proc-name" title={proc.name}>
              {proc.name}
            </span>
            <span class="proc-cpu">{formatPercent(proc.cpuUsage, 1)}</span>
            <span class="proc-mem">{formatBytes(proc.memoryBytes)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
