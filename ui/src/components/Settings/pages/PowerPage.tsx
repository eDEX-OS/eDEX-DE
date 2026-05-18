import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface PowerSettings {
  screen_timeout: number;
  suspend_timeout: number;
  suspend_enabled: boolean;
}

interface BatteryStatus {
  present: boolean;
  percentage?: number;
  state?: string;
  timeToEmpty?: string;
  timeToFull?: string;
}

export function PowerPage() {
  const [settings, setSettings] = useState<PowerSettings>({
    screen_timeout: 300,
    suspend_timeout: 600,
    suspend_enabled: true,
  });
  const [battery, setBattery] = useState<BatteryStatus>({ present: false });
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<PowerSettings>('get_power_settings').then(setSettings).catch(() => {});
    invoke<BatteryStatus>('get_battery_status').then(setBattery).catch(() => {});
  }, []);

  const save = async () => {
    try {
      await invoke('set_power_settings', { settings });
      setStatus('Saved');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Power</h2>

      {battery.present && (
        <div class="settings-section">
          <h3 class="settings-section-title">Battery</h3>
          <div class="settings-battery-bar">
            <div
              class="settings-battery-fill"
              style={{ width: `${battery.percentage ?? 0}%` }}
            />
            <span class="settings-battery-label">
              {battery.percentage?.toFixed(0) ?? '?'}%
              {battery.state ? ` · ${battery.state}` : ''}
              {battery.timeToEmpty ? ` · ${battery.timeToEmpty}` : ''}
              {battery.timeToFull ? ` · ${battery.timeToFull}` : ''}
            </span>
          </div>
        </div>
      )}

      <div class="settings-section">
        <h3 class="settings-section-title">Screen</h3>
        <div class="settings-row">
          <label class="settings-label">
            Turn off screen after (seconds, 0 = never)
          </label>
          <input
            type="number"
            min="0"
            step="60"
            class="settings-input settings-input-sm"
            value={settings.screen_timeout}
            onInput={(e) => setSettings((s) => ({
              ...s,
              screen_timeout: Number((e.target as HTMLInputElement).value),
            }))}
          />
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Suspend</h3>
        <div class="settings-row">
          <label class="settings-label">Enable Auto-Suspend</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={settings.suspend_enabled}
            onChange={(e) => setSettings((s) => ({
              ...s,
              suspend_enabled: (e.target as HTMLInputElement).checked,
            }))}
          />
        </div>
        {settings.suspend_enabled && (
          <div class="settings-row">
            <label class="settings-label">Suspend after (seconds)</label>
            <input
              type="number"
              min="60"
              step="60"
              class="settings-input settings-input-sm"
              value={settings.suspend_timeout}
              onInput={(e) => setSettings((s) => ({
                ...s,
                suspend_timeout: Number((e.target as HTMLInputElement).value),
              }))}
            />
          </div>
        )}
      </div>

      {status && <p class="settings-status">{status}</p>}
      <div class="settings-actions">
        <button class="btn btn-primary" onClick={save}>Apply</button>
      </div>
    </div>
  );
}
