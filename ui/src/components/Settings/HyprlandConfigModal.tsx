import { useState, useEffect } from 'preact/hooks';
import { generateHyprlandConfig, saveHyprlandIntegrationConfig } from '../../ipc';

interface Props {
  onClose: () => void;
}

export function HyprlandConfigModal({ onClose }: Props) {
  const [config, setConfig] = useState('');
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    generateHyprlandConfig().then(setConfig).catch(console.error);
  }, []);

  const handleSave = async () => {
    try {
      const path = await saveHyprlandIntegrationConfig();
      setSaved(path);
      setError(null);
      window.setTimeout(() => setSaved(null), 4000);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div class="modal-overlay" onClick={onClose}>
      <div class="modal hyprland-config-modal" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h2 class="modal-title">HYPRLAND INTEGRATION</h2>
          <button class="modal-close" onClick={onClose}>×</button>
        </div>
        <div class="modal-body">
          <p style={{ fontSize: '11px', color: 'var(--color-fg-muted)', marginBottom: '8px' }}>
            Add the following to your <code>hyprland.conf</code>, or click Save to write it to{' '}
            <code>~/.config/edex-de/hyprland-integration.conf</code> and source it.
          </p>
          {saved && (
            <div style={{ padding: '6px 10px', background: 'rgba(0,255,136,0.1)', border: '1px solid var(--color-success)', fontSize: '11px', color: 'var(--color-success)', marginBottom: '8px' }}>
              ✓ Saved to {saved}
            </div>
          )}
          {error && (
            <div style={{ padding: '6px 10px', background: 'rgba(255,77,77,0.1)', border: '1px solid var(--color-error)', fontSize: '11px', color: 'var(--color-error)', marginBottom: '8px' }}>
              Error: {error}
            </div>
          )}
          <pre class="config-preview">{config}</pre>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onClick={handleSave}>Save Config</button>
          <button class="btn btn-primary" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
