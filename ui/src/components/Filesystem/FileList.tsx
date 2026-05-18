import { useEffect, useState } from 'preact/hooks';
import type { FileEntry } from '../../types';
import { listDir } from '../../ipc';
import { formatBytes } from '../../utils';
import { useSettings } from '../../context';

interface FileListProps {
  initialPath?: string;
  onNavigate?: (path: string) => void;
}

export function FileList({ initialPath = '/', onNavigate }: FileListProps) {
  const { settings } = useSettings();
  const [cwd, setCwd] = useState(initialPath);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);

  const navigate = (path: string) => {
    setCwd(path);
    onNavigate?.(path);
  };

  useEffect(() => {
    setLoading(true);
    setError(null);
    listDir(cwd, !settings.hideDotfiles)
      .then((e) => {
        setEntries(e);
        setSelected(null);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [cwd, settings.hideDotfiles]);

  const handleClick = (entry: FileEntry) => {
    if (entry.isDir) navigate(entry.path);
    else setSelected(entry.path === selected ? null : entry.path);
  };

  const goUp = () => {
    const parent = cwd.split('/').slice(0, -1).join('/') || '/';
    navigate(parent);
  };

  const mimeIcon = (mime?: string, isDir = false): string => {
    if (isDir) return '📁';
    if (!mime) return '📄';
    if (mime.startsWith('image/')) return '🖼';
    if (mime.startsWith('video/')) return '🎬';
    if (mime.startsWith('audio/')) return '🎵';
    if (mime.includes('pdf')) return '📕';
    if (mime.includes('zip') || mime.includes('tar') || mime.includes('gzip')) return '📦';
    if (mime.startsWith('text/')) return '📝';
    return '📄';
  };

  return (
    <div class="filesystem-panel">
      <div class="fs-breadcrumb">
        <button class="fs-up" onClick={goUp} title="Parent directory">
          ↑
        </button>
        <span class="fs-cwd" title={cwd}>
          {cwd}
        </span>
      </div>
      {loading && <div class="fs-loading">Loading...</div>}
      {error && <div class="fs-error">{error}</div>}
      <div class={`fs-list ${settings.fsListView ? 'list-view' : 'grid-view'}`}>
        {entries.map((entry) => (
          <div
            key={entry.path}
            class={`fs-entry ${entry.isDir ? 'dir' : 'file'} ${selected === entry.path ? 'selected' : ''}`}
            onClick={() => handleClick(entry)}
            title={entry.path}
          >
            <span class="fs-icon">{mimeIcon(entry.mime, entry.isDir)}</span>
            <span class="fs-name">{entry.name}</span>
            {!entry.isDir && <span class="fs-size">{formatBytes(entry.size)}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
