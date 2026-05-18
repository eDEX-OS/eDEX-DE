import { useState, useEffect } from 'preact/hooks';
import { generateHyprlandConfig, saveHyprlandIntegrationConfig } from '../../../ipc';

export function HyprlandPage() {
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
      setTimeout(() => setSaved(null), 4000);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Hyprland</h2>
      <p class="settings-description">
        View and save the generated eDEX-DE Hyprland integration config. This config is
        automatically applied when eDEX-DE starts. Manual edits go in{' '}
        <code>~/.config/edex-de/hyprland-extra.conf</code>.
      </p>
      <div class="settings-section">
        <h3 class="settings-section-title">Generated Config</h3>
        <textarea
          class="settings-code-editor"
          value={config}
          onInput={(e) => setConfig((e.target as HTMLTextAreaElement).value)}
          rows={20}
          spellcheck={false}
        />
      </div>
      {saved && <p class="settings-status">Saved to {saved}</p>}
      {error && <p class="settings-status settings-error">{error}</p>}
      <div class="settings-actions">
        <button class="btn btn-primary" onClick={handleSave}>Save Config</button>
      </div>
    </div>
  );
}
