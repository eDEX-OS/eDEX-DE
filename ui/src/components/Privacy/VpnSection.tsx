import { useEffect, useState } from 'preact/hooks';
import type { VpnConnection } from '../../types';
import { vpnListConnections, vpnConnect, vpnDisconnect, vpnImportWireguard } from '../../ipc';

export function VpnSection() {
  const [connections, setConnections] = useState<VpnConnection[]>([]);
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [importText, setImportText] = useState('');
  const [importName, setImportName] = useState('');
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    try {
      const conns = await vpnListConnections();
      setConnections(conns);
    } catch {
      setConnections([]);
    }
  };

  useEffect(() => {
    refresh().finally(() => setLoading(false));
  }, []);

  const handleConnect = async (name: string) => {
    setError(null);
    try {
      await vpnConnect(name);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDisconnect = async (name: string) => {
    setError(null);
    try {
      await vpnDisconnect(name);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleImport = async () => {
    if (!importText.trim() || !importName.trim()) {
      setError('Profile name and config content are required.');
      return;
    }
    setError(null);
    setImporting(true);
    try {
      await vpnImportWireguard(importText, importName);
      setImportText('');
      setImportName('');
      setShowImport(false);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  };

  return (
    <div class="privacy-section">
      {loading ? (
        <div class="privacy-info-box">Loading…</div>
      ) : connections.length === 0 ? (
        <div class="privacy-info-box">
          No WireGuard or VPN connections found. Import a WireGuard config below.
        </div>
      ) : (
        <div>
          <div class="privacy-section-label">CONNECTIONS</div>
          <table class="privacy-table">
            <thead>
              <tr>
                <th>NAME</th>
                <th>TYPE</th>
                <th>STATUS</th>
                <th>ACTIONS</th>
              </tr>
            </thead>
            <tbody>
              {connections.map((c) => (
                <tr key={c.uuid}>
                  <td>{c.name}</td>
                  <td>{c.vpnType}</td>
                  <td>
                    <span class={`privacy-badge ${c.active ? 'active' : 'inactive'}`}>
                      {c.active ? 'ACTIVE' : 'INACTIVE'}
                    </span>
                  </td>
                  <td>
                    {c.active ? (
                      <button class="btn btn-secondary" onClick={() => handleDisconnect(c.name)}>
                        DISCONNECT
                      </button>
                    ) : (
                      <button class="btn btn-primary" onClick={() => handleConnect(c.name)}>
                        CONNECT
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div>
        <button
          class="btn btn-secondary"
          onClick={() => setShowImport((v) => !v)}
        >
          {showImport ? 'CANCEL IMPORT' : 'IMPORT WIREGUARD CONFIG'}
        </button>
      </div>

      {showImport && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
          <div class="privacy-section-label">IMPORT WIREGUARD</div>
          <input
            class="settings-input"
            type="text"
            placeholder="Profile name"
            value={importName}
            onInput={(e) => setImportName((e.target as HTMLInputElement).value)}
          />
          <textarea
            class="privacy-textarea"
            placeholder="Paste WireGuard .conf content here…"
            value={importText}
            onInput={(e) => setImportText((e.target as HTMLTextAreaElement).value)}
            style={{ minHeight: '120px' }}
          />
          <button class="btn btn-primary" disabled={importing} onClick={handleImport}>
            {importing ? 'IMPORTING…' : 'IMPORT'}
          </button>
        </div>
      )}

      {error && <div class="privacy-error">{error}</div>}
    </div>
  );
}
