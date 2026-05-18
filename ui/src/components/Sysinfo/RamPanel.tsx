import { useSysinfo } from '../../context';
import { formatBytes, formatPercent } from '../../utils';

export function RamPanel() {
  const { ram } = useSysinfo();

  if (!ram) return <div class="sysinfo-loading">RAM</div>;

  const usedPct = (ram.used / ram.total) * 100;
  const swapPct = ram.swapTotal > 0 ? (ram.swapUsed / ram.swapTotal) * 100 : 0;

  return (
    <div class="sysinfo-panel ram-panel">
      <div class="sysinfo-header">MEMORY</div>
      <div class="ram-row">
        <span class="ram-label">RAM</span>
        <div class="gauge-bar">
          <div class="gauge-fill" style={{ width: `${usedPct}%` }} />
        </div>
        <span class="gauge-value">{formatPercent(usedPct)}</span>
      </div>
      <div class="ram-stats">
        <span>
          {formatBytes(ram.used)} / {formatBytes(ram.total)}
        </span>
        <span>Free: {formatBytes(ram.available)}</span>
      </div>
      {ram.swapTotal > 0 && (
        <div class="ram-row">
          <span class="ram-label">SWAP</span>
          <div class="gauge-bar">
            <div class="gauge-fill swap" style={{ width: `${swapPct}%` }} />
          </div>
          <span class="gauge-value">{formatPercent(swapPct)}</span>
        </div>
      )}
    </div>
  );
}
