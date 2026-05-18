import { useState } from 'preact/hooks';
import { TorSection } from './TorSection';
import { TailscaleSection } from './TailscaleSection';
import { VpnSection } from './VpnSection';

interface PrivacyModalProps {
  onClose: () => void;
}

export function PrivacyModal({ onClose }: PrivacyModalProps) {
  const [tab, setTab] = useState<'tor' | 'tailscale' | 'vpn'>('tor');

  return (
    <div class="modal-overlay" onClick={onClose}>
      <div class="modal privacy-modal" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <span class="modal-title">PRIVACY CONTROL</span>
          <button class="modal-close" onClick={onClose}>×</button>
        </div>
        <div class="privacy-tabs">
          {(['tor', 'tailscale', 'vpn'] as const).map((t) => (
            <button
              key={t}
              class={`privacy-tab${tab === t ? ' active' : ''}`}
              onClick={() => setTab(t)}
            >
              {t.toUpperCase()}
            </button>
          ))}
        </div>
        <div class="modal-body privacy-body">
          {tab === 'tor' && <TorSection />}
          {tab === 'tailscale' && <TailscaleSection />}
          {tab === 'vpn' && <VpnSection />}
        </div>
      </div>
    </div>
  );
}
