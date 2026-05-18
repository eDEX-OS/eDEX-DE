import { useSysinfo } from '../../context';
import { formatBytes } from '../../utils';

export function NetPanel() {
  const { net } = useSysinfo();

  return (
    <div class="sysinfo-panel net-panel">
      <div class="sysinfo-header">NETWORK</div>
      {net.length === 0 && <div class="sysinfo-loading">No interfaces</div>}
      {net.map((iface) => (
        <div key={iface.name} class="net-iface">
          <div class="net-name">{iface.name}</div>
          <div class="net-stats">
            <span class="net-rx">↓ {formatBytes(iface.rxBytesPerSec)}/s</span>
            <span class="net-tx">↑ {formatBytes(iface.txBytesPerSec)}/s</span>
          </div>
          <div class="net-totals">
            <span>Total RX: {formatBytes(iface.rxBytes)}</span>
            <span>Total TX: {formatBytes(iface.txBytes)}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
