import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface Monitor {
  name: string;
  width: number;
  height: number;
  refreshRate: number;
  x: number;
  y: number;
  scale: number;
}

export function DisplayPage() {
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [selected, setSelected] = useState(0);
  const [draft, setDraft] = useState<Partial<Monitor>>({});
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<any[]>('get_display_info')
      .then((raw) => {
        const parsed: Monitor[] = raw.map((m) => ({
          name: m.name,
          width: m.width,
          height: m.height,
          refreshRate: m.refreshRate,
          x: m.x,
          y: m.y,
          scale: m.scale,
        }));
        setMonitors(parsed);
        if (parsed.length > 0) setDraft({ ...parsed[0] });
      })
      .catch(() => {});
  }, []);

  const mon = monitors[selected];

  const patch = <K extends keyof Monitor>(k: K, v: Monitor[K]) =>
    setDraft((p) => ({ ...p, [k]: v }));

  const apply = async () => {
    if (!mon) return;
    const cfg = { ...mon, ...draft };
    try {
      await invoke('set_monitor_config', { config: cfg });
      setStatus('Applied');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Display</h2>
      {monitors.length === 0 ? (
        <p class="settings-empty">No monitor info available (hyprctl required)</p>
      ) : (
        <>
          <div class="settings-section">
            <h3 class="settings-section-title">Monitor</h3>
            <div class="settings-row">
              <label class="settings-label">Active Monitor</label>
              <select
                class="settings-select"
                value={selected}
                onChange={(e) => {
                  const idx = Number((e.target as HTMLSelectElement).value);
                  setSelected(idx);
                  setDraft({ ...monitors[idx] });
                }}
              >
                {monitors.map((m, i) => (
                  <option key={m.name} value={i}>{m.name}</option>
                ))}
              </select>
            </div>
            {mon && (
              <>
                <div class="settings-row">
                  <label class="settings-label">Resolution</label>
                  <div class="settings-inline">
                    <input
                      type="number"
                      class="settings-input settings-input-sm"
                      value={draft.width ?? mon.width}
                      onInput={(e) => patch('width', Number((e.target as HTMLInputElement).value))}
                    />
                    <span class="settings-separator">×</span>
                    <input
                      type="number"
                      class="settings-input settings-input-sm"
                      value={draft.height ?? mon.height}
                      onInput={(e) => patch('height', Number((e.target as HTMLInputElement).value))}
                    />
                  </div>
                </div>
                <div class="settings-row">
                  <label class="settings-label">Refresh Rate (Hz)</label>
                  <input
                    type="number"
                    step="0.001"
                    class="settings-input settings-input-sm"
                    value={draft.refreshRate ?? mon.refreshRate}
                    onInput={(e) => patch('refreshRate', Number((e.target as HTMLInputElement).value))}
                  />
                </div>
                <div class="settings-row">
                  <label class="settings-label">Scale</label>
                  <input
                    type="number"
                    step="0.25"
                    min="0.5"
                    max="3"
                    class="settings-input settings-input-sm"
                    value={draft.scale ?? mon.scale}
                    onInput={(e) => patch('scale', Number((e.target as HTMLInputElement).value))}
                  />
                </div>
              </>
            )}
          </div>
          {status && <p class="settings-status">{status}</p>}
          <div class="settings-actions">
            <button class="btn btn-primary" onClick={apply}>Apply</button>
          </div>
        </>
      )}
    </div>
  );
}
