import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface BtDevice {
  address: string;
  name: string;
  paired: boolean;
  connected: boolean;
  device_type: string;
}

export function BluetoothPage() {
  const [available, setAvailable] = useState(false);
  const [devices, setDevices] = useState<BtDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [busy, setBusy] = useState('');
  const [status, setStatus] = useState('');

  const toast = (msg: string) => {
    setStatus(msg);
    setTimeout(() => setStatus(''), 4000);
  };

  useEffect(() => {
    invoke<boolean>('bluetooth_available').then(setAvailable).catch(() => {});
    invoke<BtDevice[]>('bluetooth_list_devices').then(setDevices).catch(() => {});
  }, []);

  const scan = async () => {
    setScanning(true);
    try {
      const found = await invoke<BtDevice[]>('bluetooth_scan');
      setDevices(found);
    } catch (e: any) {
      toast(`Scan failed: ${e}`);
    } finally {
      setScanning(false);
    }
  };

  const pair = async (addr: string) => {
    setBusy(addr);
    try {
      await invoke('bluetooth_pair', { address: addr });
      toast('Paired successfully');
      const updated = await invoke<BtDevice[]>('bluetooth_list_devices');
      setDevices(updated);
    } catch (e: any) {
      toast(`Pair failed: ${e}`);
    } finally {
      setBusy('');
    }
  };

  const connect = async (addr: string) => {
    setBusy(addr);
    try {
      await invoke('bluetooth_connect', { address: addr });
      toast('Connected');
      const updated = await invoke<BtDevice[]>('bluetooth_list_devices');
      setDevices(updated);
    } catch (e: any) {
      toast(`Connect failed: ${e}`);
    } finally {
      setBusy('');
    }
  };

  const disconnect = async (addr: string) => {
    setBusy(addr);
    try {
      await invoke('bluetooth_disconnect', { address: addr });
      toast('Disconnected');
      const updated = await invoke<BtDevice[]>('bluetooth_list_devices');
      setDevices(updated);
    } catch (e: any) {
      toast(`Disconnect failed: ${e}`);
    } finally {
      setBusy('');
    }
  };

  const remove = async (addr: string) => {
    setBusy(addr);
    try {
      await invoke('bluetooth_remove', { address: addr });
      setDevices((d) => d.filter((dev) => dev.address !== addr));
      toast('Device removed');
    } catch (e: any) {
      toast(`Remove failed: ${e}`);
    } finally {
      setBusy('');
    }
  };

  if (!available) {
    return (
      <div class="settings-page">
        <h2 class="settings-page-title">Bluetooth</h2>
        <p class="settings-empty">Bluetooth not available (bluetoothctl required)</p>
      </div>
    );
  }

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Bluetooth</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Devices</h3>
        <div class="settings-actions">
          <button class="btn btn-secondary" onClick={scan} disabled={scanning}>
            {scanning ? 'Scanning...' : 'Scan for Devices'}
          </button>
        </div>

        {devices.length === 0 && !scanning && (
          <p class="settings-empty">No devices found. Scan to discover nearby devices.</p>
        )}

        {devices.map((d) => (
          <div key={d.address} class={`settings-device-row${d.connected ? ' active' : ''}`}>
            <div class="settings-device-info">
              <span class="settings-device-name">{d.name}</span>
              <span class="settings-device-meta">{d.address} · {d.device_type}</span>
            </div>
            <div class="settings-device-actions">
              {d.connected ? (
                <button
                  class="btn btn-sm btn-secondary"
                  disabled={busy === d.address}
                  onClick={() => disconnect(d.address)}
                >
                  Disconnect
                </button>
              ) : d.paired ? (
                <button
                  class="btn btn-sm btn-primary"
                  disabled={busy === d.address}
                  onClick={() => connect(d.address)}
                >
                  Connect
                </button>
              ) : (
                <button
                  class="btn btn-sm btn-primary"
                  disabled={busy === d.address}
                  onClick={() => pair(d.address)}
                >
                  Pair
                </button>
              )}
              {d.paired && (
                <button
                  class="btn btn-sm btn-danger"
                  disabled={busy === d.address}
                  onClick={() => remove(d.address)}
                >
                  Remove
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      {status && <p class="settings-status">{status}</p>}
    </div>
  );
}
