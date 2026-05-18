import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface WifiNetwork {
  ssid: string;
  signal: number;
  security: string;
  active: boolean;
}

interface ActiveConnection {
  name: string;
  type: string;
  ip4?: string;
  dns?: string[];
}

export function NetworkPage() {
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [active, setActive] = useState<ActiveConnection | null>(null);
  const [scanning, setScanning] = useState(false);
  const [connecting, setConnecting] = useState('');
  const [password, setPassword] = useState('');
  const [selectedSsid, setSelectedSsid] = useState('');
  const [status, setStatus] = useState('');

  const refresh = async () => {
    try {
      const conn = await invoke<ActiveConnection>('get_active_connection_info');
      setActive(conn);
    } catch {}
  };

  const scan = async () => {
    setScanning(true);
    try {
      const nets = await invoke<WifiNetwork[]>('wifi_scan');
      setNetworks(nets);
    } catch (e: any) {
      setStatus(`Scan failed: ${e}`);
    } finally {
      setScanning(false);
    }
    setTimeout(() => setStatus(''), 4000);
  };

  const connect = async (ssid: string) => {
    setConnecting(ssid);
    try {
      await invoke('wifi_connect', { ssid, password });
      setStatus(`Connected to ${ssid}`);
      setPassword('');
      setSelectedSsid('');
      refresh();
    } catch (e: any) {
      setStatus(`Failed: ${e}`);
    } finally {
      setConnecting('');
    }
    setTimeout(() => setStatus(''), 5000);
  };

  const disconnect = async () => {
    try {
      await invoke('nm_disconnect');
      setActive(null);
      setStatus('Disconnected');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  useEffect(() => { refresh(); }, []);

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Network</h2>

      {active && (
        <div class="settings-section">
          <h3 class="settings-section-title">Active Connection</h3>
          <div class="settings-info-block">
            <span class="settings-info-key">Network</span>
            <span class="settings-info-val">{active.name}</span>
          </div>
          <div class="settings-info-block">
            <span class="settings-info-key">Type</span>
            <span class="settings-info-val">{active.type}</span>
          </div>
          {active.ip4 && (
            <div class="settings-info-block">
              <span class="settings-info-key">IP</span>
              <span class="settings-info-val">{active.ip4}</span>
            </div>
          )}
          {active.dns && active.dns.length > 0 && (
            <div class="settings-info-block">
              <span class="settings-info-key">DNS</span>
              <span class="settings-info-val">{active.dns.join(', ')}</span>
            </div>
          )}
          <div class="settings-actions">
            <button class="btn btn-danger btn-sm" onClick={disconnect}>Disconnect</button>
          </div>
        </div>
      )}

      <div class="settings-section">
        <h3 class="settings-section-title">Wi-Fi Networks</h3>
        <div class="settings-actions">
          <button class="btn btn-secondary" onClick={scan} disabled={scanning}>
            {scanning ? 'Scanning...' : 'Scan'}
          </button>
        </div>
        {networks.map((n) => (
          <div key={n.ssid} class={`settings-device-row${n.active ? ' active' : ''}`}>
            <div class="settings-wifi-info">
              <span class="settings-device-name">{n.ssid}</span>
              <span class="settings-wifi-meta">{n.security} · {n.signal}%</span>
            </div>
            {n.active ? (
              <span class="settings-device-badge">CONNECTED</span>
            ) : selectedSsid === n.ssid ? (
              <div class="settings-wifi-connect">
                <input
                  type="password"
                  placeholder="Password"
                  class="settings-input settings-input-sm"
                  value={password}
                  onInput={(e) => setPassword((e.target as HTMLInputElement).value)}
                />
                <button
                  class="btn btn-primary btn-sm"
                  disabled={!!connecting}
                  onClick={() => connect(n.ssid)}
                >
                  {connecting === n.ssid ? '...' : 'Connect'}
                </button>
                <button class="btn btn-secondary btn-sm" onClick={() => setSelectedSsid('')}>
                  Cancel
                </button>
              </div>
            ) : (
              <button class="btn btn-secondary btn-sm" onClick={() => setSelectedSsid(n.ssid)}>
                Connect
              </button>
            )}
          </div>
        ))}
      </div>

      {status && <p class="settings-status">{status}</p>}
    </div>
  );
}
