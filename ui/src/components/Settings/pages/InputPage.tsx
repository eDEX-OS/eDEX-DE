import { useState } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

const KB_LAYOUTS = [
  'us', 'gb', 'de', 'fr', 'es', 'it', 'ru', 'jp', 'kr', 'br',
  'pl', 'se', 'no', 'fi', 'dk', 'nl', 'be', 'ch', 'pt', 'tr',
];

export function InputPage() {
  const [layout, setLayout] = useState('us');
  const [sensitivity, setSensitivity] = useState(0);
  const [naturalScroll, setNaturalScroll] = useState(false);
  const [status, setStatus] = useState('');

  const apply = async () => {
    try {
      await invoke('set_keyboard_layout', { layout });
      await invoke('set_mouse_sensitivity', { sensitivity });
      await invoke('set_natural_scroll', { enabled: naturalScroll });
      setStatus('Applied');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Input</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Keyboard</h3>
        <div class="settings-row">
          <label class="settings-label">Layout</label>
          <select
            class="settings-select"
            value={layout}
            onChange={(e) => setLayout((e.target as HTMLSelectElement).value)}
          >
            {KB_LAYOUTS.map((l) => (
              <option key={l} value={l}>{l.toUpperCase()}</option>
            ))}
          </select>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Mouse / Touchpad</h3>
        <div class="settings-row">
          <label class="settings-label">Sensitivity ({sensitivity})</label>
          <input
            type="range"
            min="-1"
            max="1"
            step="0.1"
            class="settings-slider"
            value={sensitivity}
            onInput={(e) => setSensitivity(Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Natural Scroll</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={naturalScroll}
            onChange={(e) => setNaturalScroll((e.target as HTMLInputElement).checked)}
          />
        </div>
      </div>

      {status && <p class="settings-status">{status}</p>}
      <div class="settings-actions">
        <button class="btn btn-primary" onClick={apply}>Apply</button>
      </div>
    </div>
  );
}
