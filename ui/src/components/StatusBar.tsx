import { useEffect, useState } from 'preact/hooks';
import { useSysinfo } from '../context';
import { formatPercent } from '../utils';
import { audioAvailable, getMasterVolume, setMasterVolume, toggleMute } from '../ipc';

export function StatusBar() {
  const { cpu, ram, battery } = useSysinfo();
  const [volume, setVolume] = useState<number | null>(null);
  const [muted, setMuted] = useState(false);
  const [hasAudio, setHasAudio] = useState(false);
  const ramPercent = ram && ram.total > 0 ? formatPercent((ram.used / ram.total) * 100) : '--';

  useEffect(() => {
    let mounted = true;

    audioAvailable().then((available) => {
      if (!mounted) {
        return;
      }
      setHasAudio(available);
      if (available) {
        getMasterVolume().then((value) => mounted && setVolume(value)).catch(() => {});
      }
    });

    const id = window.setInterval(() => {
      if (hasAudio) {
        getMasterVolume().then((value) => mounted && setVolume(value)).catch(() => {});
      }
    }, 5000);

    return () => {
      mounted = false;
      window.clearInterval(id);
    };
  }, [hasAudio]);

  const handleVolumeChange = async (event: Event) => {
    const value = Number((event.target as HTMLInputElement).value);
    setVolume(value);
    await setMasterVolume(value).catch(() => {});
  };

  const handleMuteToggle = async () => {
    const nextMuted = await toggleMute().catch(() => muted);
    setMuted(nextMuted);
  };

  return (
    <div class="statusbar">
      <span class="status-item">
        CPU: <strong>{cpu ? formatPercent(cpu.totalUsage) : '--'}</strong>
      </span>
      <span class="status-item">
        RAM: <strong>{ramPercent}</strong>
      </span>
      {battery?.hasBattery && (
        <span class="status-item">
          BAT: <strong>{battery.percent ?? '--'}%{battery.isCharging ? ' ⚡' : ''}</strong>
        </span>
      )}
      {hasAudio && volume !== null && (
        <span class="status-item status-volume">
          <button class="vol-mute" onClick={handleMuteToggle} title={muted ? 'Unmute' : 'Mute'}>
            {muted ? '🔇' : volume > 50 ? '🔊' : volume > 0 ? '🔉' : '🔈'}
          </button>
          <input
            type="range"
            min="0"
            max="100"
            value={volume}
            class="vol-slider"
            onInput={handleVolumeChange}
            title={`Volume: ${volume}%`}
          />
          <span class="vol-value">{volume}%</span>
        </span>
      )}
    </div>
  );
}
