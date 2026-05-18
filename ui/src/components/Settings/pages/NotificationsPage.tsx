import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface NotificationConfig {
  font: string;
  font_size: number;
  background_color: string;
  text_color: string;
  border_color: string;
  timeout: number;
  max_visible: number;
  position: string;
}

const POSITIONS = [
  'top-right', 'top-center', 'top-left',
  'bottom-right', 'bottom-center', 'bottom-left',
];

export function NotificationsPage() {
  const [config, setConfig] = useState<NotificationConfig>({
    font: 'monospace',
    font_size: 12,
    background_color: '#1a1a2e',
    text_color: '#00ff99',
    border_color: '#00ff9966',
    timeout: 5000,
    max_visible: 5,
    position: 'top-right',
  });
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<NotificationConfig>('get_notification_config').then(setConfig).catch(() => {});
  }, []);

  const patch = <K extends keyof NotificationConfig>(k: K, v: NotificationConfig[K]) =>
    setConfig((c) => ({ ...c, [k]: v }));

  const save = async () => {
    try {
      await invoke('set_notification_config', { config });
      setStatus('Saved and reloaded');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Notifications</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Appearance</h3>
        <div class="settings-row">
          <label class="settings-label">Font</label>
          <input
            class="settings-input"
            value={config.font}
            onInput={(e) => patch('font', (e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Font Size</label>
          <input
            type="number"
            min="8"
            max="24"
            class="settings-input settings-input-sm"
            value={config.font_size}
            onInput={(e) => patch('font_size', Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Background Color</label>
          <div class="settings-color-row">
            <input
              type="color"
              class="settings-color-picker"
              value={config.background_color.slice(0, 7)}
              onInput={(e) => patch('background_color', (e.target as HTMLInputElement).value)}
            />
            <input
              class="settings-input settings-input-sm"
              value={config.background_color}
              onInput={(e) => patch('background_color', (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>
        <div class="settings-row">
          <label class="settings-label">Text Color</label>
          <div class="settings-color-row">
            <input
              type="color"
              class="settings-color-picker"
              value={config.text_color.slice(0, 7)}
              onInput={(e) => patch('text_color', (e.target as HTMLInputElement).value)}
            />
            <input
              class="settings-input settings-input-sm"
              value={config.text_color}
              onInput={(e) => patch('text_color', (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>
        <div class="settings-row">
          <label class="settings-label">Border Color</label>
          <div class="settings-color-row">
            <input
              type="color"
              class="settings-color-picker"
              value={config.border_color.slice(0, 7)}
              onInput={(e) => patch('border_color', (e.target as HTMLInputElement).value)}
            />
            <input
              class="settings-input settings-input-sm"
              value={config.border_color}
              onInput={(e) => patch('border_color', (e.target as HTMLInputElement).value)}
            />
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Behavior</h3>
        <div class="settings-row">
          <label class="settings-label">Timeout (ms)</label>
          <input
            type="number"
            min="1000"
            step="500"
            class="settings-input settings-input-sm"
            value={config.timeout}
            onInput={(e) => patch('timeout', Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Max Visible</label>
          <input
            type="number"
            min="1"
            max="10"
            class="settings-input settings-input-sm"
            value={config.max_visible}
            onInput={(e) => patch('max_visible', Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Position</label>
          <select
            class="settings-select"
            value={config.position}
            onChange={(e) => patch('position', (e.target as HTMLSelectElement).value)}
          >
            {POSITIONS.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </div>
      </div>

      {status && <p class="settings-status">{status}</p>}
      <div class="settings-actions">
        <button class="btn btn-primary" onClick={save}>Apply</button>
      </div>
    </div>
  );
}
