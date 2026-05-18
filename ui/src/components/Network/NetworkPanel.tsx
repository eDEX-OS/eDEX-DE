import { useState, useEffect, useCallback } from 'preact/hooks';
import {
  networkAvailable,
  listConnections,
  wifiScan,
  getActiveConnectionInfo,
  wifiConnect,
} from '../../ipc';
import type { NetworkConnection, WifiNetwork } from '../../ipc';

const NETWORK_UNAVAILABLE_MESSAGE = 'NetworkManager not available — install network-manager / nmcli to enable WIFI controls.';

function formatNetworkError(error: unknown) {
  const message = String(error);

  if (message.includes('nmcli not found') || message.includes('No such file or directory')) {
    return NETWORK_UNAVAILABLE_MESSAGE;
  }

  return `NetworkManager unavailable — ${message}`;
}

export function NetworkPanel() {
  const [available, setAvailable] = useState(false);
  const [connections, setConnections] = useState<NetworkConnection[]>([]);
  const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
  const [activeInfo, setActiveInfo] = useState<Record<string, string>>({});
  const [statusMessage, setStatusMessage] = useState<string>(NETWORK_UNAVAILABLE_MESSAGE);
  const [scanning, setScanning] = useState(false);
  const [tab, setTab] = useState<'connections' | 'wifi'>('connections');
  const [connectSsid, setConnectSsid] = useState<string | null>(null);
  const [connectPw, setConnectPw] = useState('');

  const refreshConnections = useCallback(() => {
    Promise.all([listConnections(), getActiveConnectionInfo()])
      .then(([nextConnections, info]) => {
        setConnections(nextConnections);
        setActiveInfo(info);
        setStatusMessage('');
      })
      .catch((error) => {
        console.error(error);
        setAvailable(false);
        setConnections([]);
        setActiveInfo({});
        setStatusMessage(formatNetworkError(error));
      });
  }, []);

  useEffect(() => {
    networkAvailable()
      .then((isAvailable) => {
        setAvailable(isAvailable);
        if (isAvailable) {
          setStatusMessage('');
          refreshConnections();
          return;
        }

        setStatusMessage(NETWORK_UNAVAILABLE_MESSAGE);
      })
      .catch((error) => {
        console.error(error);
        setAvailable(false);
        setStatusMessage(formatNetworkError(error));
      });
  }, [refreshConnections]);

  const scanWifiNetworks = useCallback(() => {
    setScanning(true);
    wifiScan()
      .then((networks) => {
        setWifiNetworks(networks);
        setStatusMessage('');
      })
      .catch((error) => {
        console.error(error);
        setAvailable(false);
        setStatusMessage(formatNetworkError(error));
      })
      .finally(() => setScanning(false));
  }, []);

  const handleConnect = async (ssid: string) => {
    try {
      await wifiConnect(ssid, connectPw || undefined);
      setConnectSsid(null);
      setConnectPw('');
      setStatusMessage('');
      refreshConnections();
      scanWifiNetworks();
    } catch (error) {
      console.error(error);
      setStatusMessage(formatNetworkError(error));
    }
  };

  if (!available) {
    return (
      <div class="sysinfo-panel net-panel">
        <div class="sysinfo-header">NETWORK</div>
        <div class="sysinfo-unavailable">
          <div class="sysinfo-unavailable-title">WIFI BACKEND OFFLINE</div>
          <div class="sysinfo-unavailable-message">{statusMessage}</div>
          <div class="sysinfo-unavailable-hint">Install and start NetworkManager to populate saved connections and Wi-Fi scans.</div>
        </div>
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
      {!!statusMessage && <div class="sysinfo-inline-status">{statusMessage}</div>}
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
          {connections.length === 0 && <div class="svc-empty">No saved NetworkManager profiles detected.</div>}
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
          {!scanning && wifiNetworks.length === 0 && (
            <div class="svc-empty">No Wi-Fi networks detected yet. Trigger a scan to refresh nearby access points.</div>
          )}
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
