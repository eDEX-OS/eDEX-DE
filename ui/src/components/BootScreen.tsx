import { useEffect, useState } from 'preact/hooks';

interface BootScreenProps {
  onComplete: () => void;
  skip?: boolean;
}

const BOOT_LINES = [
  'eDEX-DE v0.4.0',
  'Initializing Rust core...',
  'Loading settings...',
  'Connecting to Hyprland IPC...',
  'Spawning terminal daemon...',
  'Boot complete.',
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
    const id = window.setInterval(() => {
      setLines((prev) => [...prev, BOOT_LINES[i]]);
      i += 1;

      if (i >= BOOT_LINES.length) {
        window.clearInterval(id);
        window.setTimeout(() => {
          setDone(true);
          window.setTimeout(onComplete, 500);
        }, 600);
      }
    }, 200);

    return () => window.clearInterval(id);
  }, [skip, onComplete]);

  if (done) return null;

  return (
    <div class="boot-screen">
      <div class="boot-logo">eDEX-DE</div>
      <div class="boot-lines">
        {lines.map((line, index) => (
          <div key={`${line}-${index}`} class="boot-line">
            {line}
          </div>
        ))}
      </div>
    </div>
  );
}
