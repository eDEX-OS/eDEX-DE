import { useCallback, useEffect, useRef, useState } from 'preact/hooks';
import type { AppEntry } from '../../types';
import { launchApp, searchApps } from '../../ipc';

interface LauncherOverlayProps {
  onClose: () => void;
}

export function LauncherOverlay({ onClose }: LauncherOverlayProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<{ app: AppEntry; score: number }[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    searchApps('')
      .then((r) => {
        setResults(r);
        setSelected(0);
      })
      .catch(console.error);
  }, []);

  useEffect(() => {
    setLoading(true);
    const timeout = window.setTimeout(() => {
      searchApps(query)
        .then((r) => {
          setResults(r);
          setSelected(0);
        })
        .catch(console.error)
        .finally(() => setLoading(false));
    }, 80);
    return () => window.clearTimeout(timeout);
  }, [query]);

  const handleLaunch = useCallback((exec: string) => {
    launchApp(exec).catch(console.error);
    onClose();
  }, [onClose]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    switch (e.key) {
      case 'Escape':
        onClose();
        break;
      case 'ArrowDown':
        e.preventDefault();
        setSelected((s) => Math.min(s + 1, results.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
        break;
      case 'Enter':
        if (results[selected]) {
          handleLaunch(results[selected].app.exec);
        }
        break;
      default:
        break;
    }
  }, [handleLaunch, onClose, results, selected]);

  return (
    <div class="launcher-overlay" onClick={onClose}>
      <div class="launcher-modal" onClick={(e) => e.stopPropagation()}>
        <div class="launcher-header">
          <span class="launcher-icon">⌨</span>
          <input
            ref={inputRef}
            class="launcher-input"
            type="text"
            placeholder="Search applications..."
            value={query}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
            onKeyDown={handleKeyDown}
            autocomplete="off"
            spellcheck={false}
          />
          {loading && <span class="launcher-spinner">◌</span>}
        </div>

        <div class="launcher-results">
          {results.length === 0 && (
            <div class="launcher-empty">
              {query ? 'No applications found' : 'No applications installed'}
            </div>
          )}
          {results.map(({ app }, i) => (
            <div
              key={app.desktopFile}
              class={`launcher-item ${i === selected ? 'selected' : ''}`}
              onClick={() => handleLaunch(app.exec)}
              onMouseEnter={() => setSelected(i)}
            >
              <div class="launcher-item-icon">
                {app.icon ? (
                  <img
                    src={`/usr/share/icons/hicolor/48x48/apps/${app.icon}.png`}
                    alt=""
                    class="app-icon-img"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = 'none';
                    }}
                  />
                ) : (
                  <span class="app-icon-fallback">▶</span>
                )}
              </div>
              <div class="launcher-item-info">
                <div class="launcher-item-name">{app.name}</div>
                {app.comment && <div class="launcher-item-desc">{app.comment}</div>}
              </div>
              {app.categories.length > 0 && (
                <div class="launcher-item-cats">
                  {app.categories.slice(0, 2).map((category) => (
                    <span key={category} class="launcher-cat">{category}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>

        <div class="launcher-footer">
          <span>↑↓ navigate</span>
          <span>↵ launch</span>
          <span>Esc close</span>
          <span class="launcher-hint">Alt+Space or Meta (Hyprland)</span>
        </div>
      </div>
    </div>
  );
}
