import { useState } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { TorSection } from '../../Privacy/TorSection';
import { TailscaleSection } from '../../Privacy/TailscaleSection';
import { VpnSection } from '../../Privacy/VpnSection';

type Tab = 'fingerprint' | 'tor' | 'tailscale' | 'vpn';

export function SecurityPrivacyPage() {
  const [tab, setTab] = useState<Tab>('fingerprint');

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Security & Privacy</h2>

      <div class="settings-subtabs">
        {(['fingerprint', 'tor', 'tailscale', 'vpn'] as Tab[]).map((t) => (
          <button
            key={t}
            class={`settings-subtab${tab === t ? ' active' : ''}`}
            onClick={() => setTab(t)}
          >
            {t.toUpperCase()}
          </button>
        ))}
      </div>

      <div class="settings-subtab-content">
        {tab === 'fingerprint' && <FingerprintSection />}
        {tab === 'tor' && <TorSection />}
        {tab === 'tailscale' && <TailscaleSection />}
        {tab === 'vpn' && <VpnSection />}
      </div>
    </div>
  );
}

function FingerprintSection() {
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);

  const enroll = async () => {
    setBusy(true);
    try {
      const result = await invoke<string>('fprintd_status');
      setStatus(String(result));
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const verify = async () => {
    setBusy(true);
    try {
      const result = await invoke<string>('fprintd_verify');
      setStatus(String(result));
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <p class="settings-description">
        Manage fingerprint enrollment via fprintd. Requires a compatible fingerprint reader.
      </p>
      <div class="settings-actions">
        <button class="btn btn-primary" onClick={enroll} disabled={busy}>
          Check Status
        </button>
        <button class="btn btn-secondary" onClick={verify} disabled={busy}>
          Verify Fingerprint
        </button>
      </div>
      {status && <pre class="settings-output">{status}</pre>}
    </div>
  );
}
