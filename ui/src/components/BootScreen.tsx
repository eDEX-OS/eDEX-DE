import { useEffect, useState } from 'preact/hooks';

interface BootScreenProps {
  onComplete: () => void;
  skip?: boolean;
}

const BOOT_LOGO = 'eDEX-DE';
const BOOT_LINES = [
  'eDEX-DE v1.1.3',
  'Running Phase 12 documentation sync...',
  'Initializing Rust core services...',
  'Loading Hyprland workspace bindings...',
  'Starting terminal PTY bridge...',
  'Checking NetworkManager and systemd hooks...',
  'Boot sequence complete.',
];

export function BootScreen({ onComplete, skip = false }: BootScreenProps) {
  const [lines, setLines] = useState<string[]>([]);
  const [done, setDone] = useState(false);

  useEffect(() => {
    if (skip) {
      onComplete();
      return;
    }

    let i = 0;
    let finishTimeout: number | undefined;
    let completeTimeout: number | undefined;

    const id = window.setInterval(() => {
      setLines((prev) => [...prev, BOOT_LINES[i]]);
      i += 1;

      if (i >= BOOT_LINES.length) {
        window.clearInterval(id);
        finishTimeout = window.setTimeout(() => {
          setDone(true);
          completeTimeout = window.setTimeout(onComplete, 500);
        }, 600);
      }
    }, 220);

    return () => {
      window.clearInterval(id);
      if (finishTimeout) window.clearTimeout(finishTimeout);
      if (completeTimeout) window.clearTimeout(completeTimeout);
    };
  }, [skip, onComplete]);

  if (done) return null;

  return (
    <div class="boot-screen">
      <div class="boot-scanline" aria-hidden="true" />
      <div class="boot-logo" aria-label={BOOT_LOGO}>
        {BOOT_LOGO.split('').map((char, index) => (
          <span
            key={`${char}-${index}`}
            class="boot-logo-char"
            style={{ animationDelay: `${index * 60}ms` }}
          >
            {char}
          </span>
        ))}
      </div>
      <div class="boot-lines">
        {lines.map((line, index) => (
          <div key={`${line}-${index}`} class="boot-line" style={{ animationDelay: `${index * 90}ms` }}>
            {line}
          </div>
        ))}
      </div>
    </div>
  );
}
