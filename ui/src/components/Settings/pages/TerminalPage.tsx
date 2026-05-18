import { useState } from 'preact/hooks';
import { useSettings } from '../../../context';
import type { Settings } from '../../../types';

export function TerminalPage() {
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
      <h2 class="settings-page-title">Terminal</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Shell</h3>
        <div class="settings-row">
          <label class="settings-label">Shell Binary</label>
          <input
            class="settings-input"
            value={val('shell')}
            onInput={(e) => patch('shell', (e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Shell Arguments</label>
          <input
            class="settings-input"
            value={val('shellArgs')}
            onInput={(e) => patch('shellArgs', (e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Starting Directory</label>
          <input
            class="settings-input"
            value={val('cwd')}
            onInput={(e) => patch('cwd', (e.target as HTMLInputElement).value)}
          />
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Files</h3>
        <div class="settings-row">
          <label class="settings-label">Hide Dotfiles</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={val('hideDotfiles')}
            onChange={(e) => patch('hideDotfiles', (e.target as HTMLInputElement).checked)}
          />
        </div>
      </div>

      <div class="settings-actions">
        <button class="btn btn-primary" onClick={save}>Apply</button>
      </div>
    </div>
  );
}
