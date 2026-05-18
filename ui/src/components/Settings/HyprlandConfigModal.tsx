import { useState, useEffect } from 'preact/hooks';
import { generateHyprlandConfig } from '../../ipc';

interface HyprlandConfigModalProps {
  onClose: () => void;
}

export function HyprlandConfigModal({ onClose }: HyprlandConfigModalProps) {
  const [config, setConfig] = useState('');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    generateHyprlandConfig().then(setConfig).catch(console.error);
  }, []);

  const copy = () => {
    navigator.clipboard.writeText(config).then(() => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div class="modal-overlay" onClick={onClose}>
      <div class="modal" style={{ maxWidth: '700px' }} onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <span class="modal-title">HYPRLAND CONFIG</span>
          <button class="modal-close" onClick={onClose}>×</button>
        </div>
        <div class="modal-body">
          <p style={{ fontSize: '12px', color: 'var(--color-fg-muted)', marginBottom: '12px' }}>
            Add these lines to <code style={{ color: 'var(--color-accent)' }}>~/.config/hypr/hyprland.conf</code>
          </p>
          <pre class="config-preview">{config}</pre>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onClick={onClose}>Close</button>
          <button class="btn btn-primary" onClick={copy}>
            {copied ? 'Copied!' : 'Copy to Clipboard'}
          </button>
        </div>
      </div>
    </div>
  );
}
