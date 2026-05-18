import { useEffect, useState } from 'preact/hooks';
import type { TorStatus, TorMode } from '../../types';
import {
  torAvailable, torStatus, torGetBridges, torSetMode,
  torRequestBridges, torSetBridges,
} from '../../ipc';

export function TorSection() {
  const [available, setAvailable] = useState(false);
  const [status, setStatus] = useState<TorStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [bridges, setBridges] = useState('');
  const [bridgeType, setBridgeType] = useState<'obfs4' | 'snowflake' | 'vanilla'>('obfs4');
  const [requestingBridges, setRequestingBridges] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    try {
      const s = await torStatus();
      setStatus(s);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    let mounted = true;
    (async () => {
      const avail = await torAvailable();
      if (!mounted) return;
      setAvailable(avail);
      if (avail) {
        await refresh();
        const b = await torGetBridges().catch(() => []);
        if (mounted) setBridges(b.map((l) => l.replace(/^Bridge /, '')).join('\n'));
      }
      setLoading(false);
    })();

    const id = window.setInterval(() => { if (available) refresh(); }, 8000);
    return () => { mounted = false; window.clearInterval(id); };
  }, []);

  const setMode = async (mode: TorMode) => {
    setError(null);
    setLoading(true);
    try {
      await torSetMode(mode);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const requestBridges = async () => {
    setRequestingBridges(true);
    setError(null);
    try {
      const list = await torRequestBridges(bridgeType);
      setBridges(list.map((l) => l.replace(/^Bridge /, '')).join('\n'));
    } catch (e) {
      setError(String(e));
    } finally {
      setRequestingBridges(false);
    }
  };

  const applyBridges = async () => {
    setError(null);
    try {
      const lines = bridges.split('\n').map((l) => l.trim()).filter(Boolean);
      await torSetBridges(lines);
    } catch (e) {
      setError(String(e));
    }
  };

  if (!available) {
    return (
      <div class="privacy-section">
        <p class="privacy-info-box">Tor is not installed. Install the <code>tor</code> package.</p>
      </div>
    );
  }

  const mode = status?.mode ?? 'off';

  return (
    <div class="privacy-section">
      <div class="privacy-status-row">
        <span class={`privacy-badge ${status?.active ? 'active' : 'inactive'}`}>
          {status?.active ? 'ACTIVE' : 'INACTIVE'}
        </span>
        <span class={`privacy-badge ${mode !== 'off' ? 'active' : 'inactive'}`}>
          {mode === 'off' ? 'OFF' : mode === 'socks5' ? 'SOCKS5' : 'TRANSPARENT PROXY'}
        </span>
        {status?.socksReachable && (
          <span class="privacy-badge active">SOCKS REACHABLE</span>
        )}
      </div>

      <div>
        <div class="privacy-section-label">MODE</div>
        <div class="privacy-mode-btns">
          {(['off', 'socks5', 'transparent'] as TorMode[]).map((m) => (
            <button
              key={m}
              class={`privacy-mode-btn${mode === m ? ' active' : ''}`}
              disabled={loading}
              onClick={() => setMode(m)}
            >
              {m === 'off' ? 'OFF' : m === 'socks5' ? 'SOCKS5' : 'TRANSPARENT PROXY'}
            </button>
          ))}
        </div>
      </div>

      {mode !== 'off' && (
        <div class="privacy-info-box">SOCKS5 proxy: 127.0.0.1:9050</div>
      )}

      <div>
        <div class="privacy-section-label">TOR BRIDGES</div>
        <textarea
          class="privacy-textarea"
          value={bridges}
          onInput={(e) => setBridges((e.target as HTMLTextAreaElement).value)}
          placeholder="One bridge per line (without 'Bridge ' prefix)"
        />
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', alignItems: 'center' }}>
          <select
            class="settings-select"
            value={bridgeType}
            onChange={(e) => setBridgeType((e.target as HTMLSelectElement).value as typeof bridgeType)}
          >
            <option value="obfs4">obfs4</option>
            <option value="snowflake">snowflake</option>
            <option value="vanilla">vanilla</option>
          </select>
          <button class="btn btn-secondary" disabled={requestingBridges} onClick={requestBridges}>
            {requestingBridges ? 'Requesting…' : 'REQUEST BRIDGES'}
          </button>
          <button class="btn btn-primary" onClick={applyBridges}>
            APPLY BRIDGES
          </button>
        </div>
      </div>

      {error && <div class="privacy-error">{error}</div>}
    </div>
  );
}
