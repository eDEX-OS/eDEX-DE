import { useState, useEffect, useCallback } from 'preact/hooks';
import {
  networkAvailable,
  listConnections,
  wifiScan,
  getActiveConnectionInfo,
  wifiConnect,
} from '../../ipc';
import type { NetworkConnection, WifiNetwork } from '../../ipc';

export function NetworkPanel() {
  const [available, setAvailable] = useState(false);
  const [connections, setConnections] = useState<NetworkConnection[]>([]);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [activeInfo, setActiveInfo] = useState<Record<string, string>>({});
  const [scanning, setScanning] = useState(false);
  const [tab, setTab] = useState<'connections' | 'wifi'>('connections');
  const [connectSsid, setConnectSsid] = useState<string | null>(null);
  const [connectPw, setConnectPw] = useState('');

  const refreshConnections = useCallback(() => {
    listConnections().then(setConnections).catch(console.error);
    getActiveConnectionInfo().then(setActiveInfo).catch(console.error);
  }, []);

  useEffect(() => {
    networkAvailable().then((isAvailable) => {
      setAvailable(isAvailable);
      if (isAvailable) {
        refreshConnections();
      }
    });
  }, [refreshConnections]);

  const scanWifiNetworks = useCallback(() => {
    setScanning(true);
    wifiScan()
      .then(setWifiNetworks)
      .catch(console.error)
      .finally(() => setScanning(false));
  }, []);

  const handleConnect = async (ssid: string) => {
    await wifiConnect(ssid, connectPw || undefined).catch(console.error);
    setConnectSsid(null);
    setConnectPw('');
    refreshConnections();
    scanWifiNetworks();
  };

  if (!available) {
    return (
      <div class="sysinfo-panel net-panel">
        <div class="sysinfo-header">NETWORK</div>
        <div class="sysinfo-loading">NetworkManager not available</div>
      </div>
    );
  }

  return (
    <div class="network-panel">
      <div class="sysinfo-header">NETWORK</div>
      {activeInfo.connection && (
        <div class="net-active-info">
          <div class="net-active-name">{activeInfo.connection}</div>
          <div class="net-active-ip">{activeInfo.ipv4 || 'No IP'}</div>
        </div>
      )}
      <div class="sysinfo-tabs" style={{ marginTop: '8px' }}>
        <button class={`sysinfo-tab ${tab === 'connections' ? 'active' : ''}`} onClick={() => setTab('connections')}>
          SAVED
        </button>
        <button
          class={`sysinfo-tab ${tab === 'wifi' ? 'active' : ''}`}
          onClick={() => {
            setTab('wifi');
            if (!wifiNetworks.length) {
              scanWifiNetworks();
            }
          }}
        >
          WIFI
        </button>
      </div>

      {tab === 'connections' && (
        <div class="net-conn-list">
          {connections.map((connection) => (
            <div key={connection.uuid} class={`net-conn ${connection.active ? 'active' : ''}`}>
              <span class="net-conn-name">{connection.name}</span>
              <span class="net-conn-type">{connection.connType}</span>
              <span
                class="net-conn-state"
                style={{ color: connection.active ? 'var(--color-success)' : 'var(--color-fg-muted)' }}
              >
                {connection.active ? '●' : '○'}
              </span>
            </div>
          ))}
        </div>
      )}

      {tab === 'wifi' && (
        <div class="net-wifi-list">
          <button
            class="btn btn-secondary"
            style={{ fontSize: '10px', padding: '3px 8px', marginBottom: '8px' }}
            onClick={scanWifiNetworks}
            disabled={scanning}
          >
            {scanning ? 'Scanning...' : 'Scan'}
          </button>
          {wifiNetworks.map((network) => (
            <div key={network.ssid} class={`net-wifi-row ${network.inUse ? 'in-use' : ''}`}>
              <div class="net-wifi-info">
                <span class="net-wifi-ssid">{network.ssid || '(hidden)'}</span>
                <div class="net-wifi-signal">
                  <div
                    class="signal-bar"
                    style={{
                      width: `${network.signal}%`,
                      background:
                        network.signal > 60
                          ? 'var(--color-success)'
                          : network.signal > 30
                            ? 'var(--color-warning)'
                            : 'var(--color-error)',
                    }}
                  />
                </div>
              </div>
              {!network.inUse &&
                (connectSsid === network.ssid ? (
                  <div class="net-wifi-connect">
                    <input
                      class="settings-input"
                      style={{ fontSize: '10px', padding: '2px 6px' }}
                      type="password"
                      placeholder="Password"
                      value={connectPw}
                      onInput={(event) => setConnectPw((event.target as HTMLInputElement).value)}
                    />
                    <button class="svc-btn start" onClick={() => handleConnect(network.ssid)}>
                      ▶
                    </button>
                    <button class="svc-btn stop" onClick={() => setConnectSsid(null)}>
                      ×
                    </button>
                  </div>
                ) : (
                  <button class="svc-btn start" style={{ fontSize: '9px' }} onClick={() => setConnectSsid(network.ssid)}>
                    Connect
                  </button>
                ))}
              {network.inUse && <span style={{ fontSize: '10px', color: 'var(--color-success)' }}>Connected</span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
