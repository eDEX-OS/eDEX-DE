import { useEffect, useState } from 'preact/hooks';
import { open } from '@tauri-apps/plugin-shell';
import type { TailscaleStatus } from '../../types';
import {
  tailscaleAvailable, tailscaleStatus, tailscaleLogin,
  tailscaleLogout, tailscaleUp, tailscaleDown, tailscaleSetExitNode,
} from '../../ipc';

export function TailscaleSection() {
  const [available, setAvailable] = useState(false);
  const [status, setStatus] = useState<TailscaleStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [authUrl, setAuthUrl] = useState<string | null>(null);
  const [selectedExitNode, setSelectedExitNode] = useState('');
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    try {
      const s = await tailscaleStatus();
      setStatus(s);
      // Keep selectedExitNode in sync with the active exit node
      const active = s.peers.find((p) => p.exitNode);
      if (active) setSelectedExitNode(active.tailscaleIps[0] ?? '');
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    let mounted = true;
    (async () => {
      const avail = await tailscaleAvailable();
      if (!mounted) return;
      setAvailable(avail);
      if (avail) await refresh();
      setLoading(false);
    })();

    const id = window.setInterval(() => { if (available) refresh(); }, 10000);
    return () => { mounted = false; window.clearInterval(id); };
  }, []);

  const handleLogin = async () => {
    setError(null);
    setLoading(true);
    try {
      const url = await tailscaleLogin();
      setAuthUrl(url);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleLogout = async () => {
    setError(null);
    try {
      await tailscaleLogout();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleConnect = async () => {
    setError(null);
    setLoading(true);
    try {
      await tailscaleUp();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleDisconnect = async () => {
    setError(null);
    try {
      await tailscaleDown();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleExitNodeChange = async (ip: string) => {
    setSelectedExitNode(ip);
    try {
      await tailscaleSetExitNode(ip || undefined);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  if (!available) {
    return (
      <div class="privacy-section">
        <p class="privacy-info-box">Tailscale is not installed.</p>
      </div>
    );
  }

  const state = status?.backendState ?? 'Stopped';
  const badgeClass = state === 'Running' ? 'active' : state === 'NeedsLogin' ? 'warning' : 'inactive';
  const exitNodePeers = status?.peers.filter((p) => p.exitNodeOption) ?? [];

  return (
    <div class="privacy-section">
      <div class="privacy-status-row">
        <span class={`privacy-badge ${badgeClass}`}>{state.toUpperCase()}</span>
        {status?.version && (
          <span class="privacy-badge inactive">v{status.version}</span>
        )}
      </div>

      {(state === 'NeedsLogin' || !status?.selfNode) && (
        <div>
          <div class="privacy-section-label">AUTHENTICATION</div>
          {authUrl ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              <div class="privacy-info-box" style={{ wordBreak: 'break-all' }}>{authUrl}</div>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <button class="btn btn-primary" onClick={() => open(authUrl)}>
                  OPEN IN BROWSER
                </button>
                <button class="btn btn-secondary" onClick={() => setAuthUrl(null)}>
                  DISMISS
                </button>
              </div>
            </div>
          ) : (
            <button class="btn btn-primary" disabled={loading} onClick={handleLogin}>
              LOGIN
            </button>
          )}
        </div>
      )}

      {state === 'Running' && status?.selfNode && (
        <>
          <div>
            <div class="privacy-section-label">THIS NODE</div>
            <div class="privacy-info-box">
              <strong>{status.selfNode.hostname}</strong>
              {' — '}
              {status.selfNode.tailscaleIps.join(', ')}
            </div>
          </div>

          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button class="btn btn-secondary" onClick={handleDisconnect}>DISCONNECT</button>
            <button class="btn btn-secondary" onClick={handleLogout}>LOGOUT</button>
          </div>

          {exitNodePeers.length > 0 && (
            <div>
              <div class="privacy-section-label">EXIT NODE</div>
              <select
                class="settings-select"
                value={selectedExitNode}
                onChange={(e) => handleExitNodeChange((e.target as HTMLSelectElement).value)}
              >
                <option value="">None</option>
                {exitNodePeers.map((p) => (
                  <option key={p.id} value={p.tailscaleIps[0] ?? ''}>
                    {p.hostname} ({p.tailscaleIps[0]})
                  </option>
                ))}
              </select>
            </div>
          )}

          {status.peers.length > 0 && (
            <div>
              <div class="privacy-section-label">PEERS</div>
              <table class="privacy-table">
                <thead>
                  <tr>
                    <th>HOSTNAME</th>
                    <th>IP</th>
                    <th>OS</th>
                    <th>STATUS</th>
                  </tr>
                </thead>
                <tbody>
                  {status.peers.map((p) => (
                    <tr key={p.id} class={p.exitNode ? 'exit-node' : ''}>
                      <td>{p.hostname}{p.exitNode ? ' ★' : ''}</td>
                      <td>{p.tailscaleIps[0] ?? '—'}</td>
                      <td>{p.os}</td>
                      <td class={p.online ? 'privacy-peer-online' : 'privacy-peer-offline'}>
                        {p.online ? 'ONLINE' : 'OFFLINE'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {state === 'Stopped' && (
        <button class="btn btn-primary" disabled={loading} onClick={handleConnect}>
          CONNECT
        </button>
      )}

      {error && <div class="privacy-error">{error}</div>}
    </div>
  );
}
