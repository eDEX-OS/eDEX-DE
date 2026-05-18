import { useState } from 'preact/hooks';
import { useSettings } from '../../context';
import type { Settings } from '../../types';

interface SettingsModalProps {
  onClose: () => void;
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const { settings, updateSettings } = useSettings();
  const [draft, setDraft] = useState<Settings>({ ...settings });
  const [saving, setSaving] = useState(false);

  const patch = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const save = async () => {
    setSaving(true);
    await updateSettings(draft);
    setSaving(false);
    onClose();
  };

  return (
    <div class="modal-overlay" onClick={onClose}>
      <div class="modal settings-modal" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <span class="modal-title">SETTINGS</span>
          <button class="modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div class="modal-body">
          <div class="settings-group">
            <label class="settings-label">Shell</label>
            <input
              class="settings-input"
              value={draft.shell}
              onInput={(e) => patch('shell', (e.target as HTMLInputElement).value)}
            />
          </div>
          <div class="settings-group">
            <label class="settings-label">Shell Arguments</label>
            <input
              class="settings-input"
              value={draft.shellArgs}
              onInput={(e) => patch('shellArgs', (e.target as HTMLInputElement).value)}
            />
          </div>
          <div class="settings-group">
            <label class="settings-label">Working Directory</label>
            <input
              class="settings-input"
              value={draft.cwd}
              onInput={(e) => patch('cwd', (e.target as HTMLInputElement).value)}
            />
          </div>
          <div class="settings-group">
            <label class="settings-label">Theme</label>
            <select
              class="settings-select"
              value={draft.theme}
              onChange={(e) => patch('theme', (e.target as HTMLSelectElement).value)}
            >
              <option value="tron">Tron</option>
              <option value="default">Default</option>
            </select>
          </div>
          <div class="settings-group">
            <label class="settings-label">Terminal Font Size</label>
            <input
              type="number"
              min="8"
              max="32"
              class="settings-input"
              value={draft.termFontSize}
              onInput={(e) => patch('termFontSize', Number((e.target as HTMLInputElement).value))}
            />
          </div>
          <div class="settings-group">
            <label class="settings-label">Clock Format</label>
            <select
              class="settings-select"
              value={String(draft.clockHours)}
              onChange={(e) => patch('clockHours', Number((e.target as HTMLSelectElement).value))}
            >
              <option value="24">24-hour</option>
              <option value="12">12-hour</option>
            </select>
          </div>
          <div class="settings-group settings-toggle">
            <label class="settings-label">Audio</label>
            <input
              type="checkbox"
              checked={draft.audio}
              onChange={(e) => patch('audio', (e.target as HTMLInputElement).checked)}
            />
          </div>
          <div class="settings-group settings-toggle">
            <label class="settings-label">Skip Boot Animation</label>
            <input
              type="checkbox"
              checked={draft.nointro}
              onChange={(e) => patch('nointro', (e.target as HTMLInputElement).checked)}
            />
          </div>
          <div class="settings-group settings-toggle">
            <label class="settings-label">Hide Dotfiles</label>
            <input
              type="checkbox"
              checked={draft.hideDotfiles}
              onChange={(e) => patch('hideDotfiles', (e.target as HTMLInputElement).checked)}
            />
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button class="btn btn-primary" onClick={save} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
}
