import { useState } from 'preact/hooks';
import { useSettings } from '../../../context';
import type { Settings } from '../../../types';

const THEMES = ['tron', 'default', 'matrix', 'amber', 'cyan'];

export function AppearancePage() {
  const { settings, updateSettings } = useSettings();
  const [draft, setDraft] = useState<Partial<Settings>>({});

  const val = <K extends keyof Settings>(k: K): Settings[K] =>
    (k in draft ? draft[k] : settings[k]) as Settings[K];

  const patch = <K extends keyof Settings>(k: K, v: Settings[K]) =>
    setDraft((p) => ({ ...p, [k]: v }));

  const save = async () => {
    await updateSettings({ ...settings, ...draft });
    setDraft({});
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Appearance</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Theme</h3>
        <div class="settings-row">
          <label class="settings-label">Color Theme</label>
          <select
            class="settings-select"
            value={val('theme')}
            onChange={(e) => patch('theme', (e.target as HTMLSelectElement).value)}
          >
            {THEMES.map((t) => (
              <option key={t} value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
            ))}
          </select>
        </div>
        <div class="settings-row">
          <label class="settings-label">Terminal Font Size</label>
          <input
            type="number"
            min="8"
            max="32"
            class="settings-input settings-input-sm"
            value={val('termFontSize')}
            onInput={(e) => patch('termFontSize', Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Clock Format</label>
          <select
            class="settings-select"
            value={String(val('clockHours'))}
            onChange={(e) => patch('clockHours', Number((e.target as HTMLSelectElement).value))}
          >
            <option value="24">24-hour</option>
            <option value="12">12-hour</option>
          </select>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Boot</h3>
        <div class="settings-row">
          <label class="settings-label">Skip Boot Animation</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={val('nointro')}
            onChange={(e) => patch('nointro', (e.target as HTMLInputElement).checked)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Boot Sound</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={val('audio')}
            onChange={(e) => patch('audio', (e.target as HTMLInputElement).checked)}
          />
        </div>
      </div>

      <div class="settings-actions">
        <button class="btn btn-primary" onClick={save}>Apply</button>
      </div>
    </div>
  );
}
