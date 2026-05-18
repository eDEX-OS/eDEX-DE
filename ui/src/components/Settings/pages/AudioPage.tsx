import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface AudioSink {
  name: string;
  description: string;
  volume: number;
  muted: boolean;
  is_default: boolean;
}

export function AudioPage() {
  const [sinks, setSinks] = useState<AudioSink[]>([]);
  const [volume, setVolume] = useState(50);
  const [muted, setMuted] = useState(false);
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<AudioSink[]>('list_audio_sinks').then(setSinks).catch(() => {});
    invoke<number>('get_master_volume').then(setVolume).catch(() => {});
  }, []);

  const setVol = async (v: number) => {
    setVolume(v);
    try {
      await invoke('set_master_volume', { volume: v });
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
  };

  const toggleMute = async () => {
    try {
      await invoke('toggle_mute');
      setMuted((m) => !m);
    } catch {}
  };

  const setDefault = async (name: string) => {
    try {
      await invoke('set_default_sink', { name });
      const fresh = await invoke<AudioSink[]>('list_audio_sinks');
      setSinks(fresh);
      setStatus('Default output changed');
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
    setTimeout(() => setStatus(''), 3000);
  };

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">Audio</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">Master Volume</h3>
        <div class="settings-row">
          <label class="settings-label">Volume ({volume}%)</label>
          <input
            type="range"
            min="0"
            max="100"
            class="settings-slider"
            value={volume}
            onInput={(e) => setVol(Number((e.target as HTMLInputElement).value))}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label">Muted</label>
          <input
            type="checkbox"
            class="settings-toggle-input"
            checked={muted}
            onChange={toggleMute}
          />
        </div>
      </div>

      <div class="settings-section">
        <h3 class="settings-section-title">Output Device</h3>
        {sinks.length === 0 ? (
          <p class="settings-empty">No audio sinks found (PipeWire/PulseAudio required)</p>
        ) : (
          sinks.map((s) => (
            <div key={s.name} class={`settings-device-row${s.is_default ? ' active' : ''}`}>
              <span class="settings-device-name">{s.description || s.name}</span>
              {!s.is_default && (
                <button class="btn btn-sm btn-secondary" onClick={() => setDefault(s.name)}>
                  Set Default
                </button>
              )}
              {s.is_default && <span class="settings-device-badge">DEFAULT</span>}
            </div>
          ))
        )}
      </div>

      {status && <p class="settings-status">{status}</p>}
    </div>
  );
}
