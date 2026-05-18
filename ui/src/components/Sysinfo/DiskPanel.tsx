import { useSysinfo } from '../../context';
import { formatBytes, formatPercent } from '../../utils';

export function DiskPanel() {
  const { disks } = useSysinfo();

  return (
    <div class="sysinfo-panel disk-panel">
      <div class="sysinfo-header">STORAGE</div>
      {disks.map((disk) => {
        const usedPct = disk.totalBytes > 0
          ? ((disk.totalBytes - disk.availableBytes) / disk.totalBytes) * 100
          : 0;
        return (
          <div key={disk.mountPoint} class="disk-entry">
            <div class="disk-name">{disk.name || disk.mountPoint}</div>
            <div class="gauge-bar">
              <div class="gauge-fill" style={{ width: `${usedPct}%` }} />
            </div>
            <div class="disk-stats">
              <span>{formatPercent(usedPct)}</span>
              <span>
                {formatBytes(disk.availableBytes)} free / {formatBytes(disk.totalBytes)}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
