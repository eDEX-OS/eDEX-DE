import { useEffect, useState } from 'preact/hooks';
import { getWorkspaces, switchWorkspace, onHyprlandEvent, getActiveWindow } from '../../ipc';
import type { WorkspaceInfo, ActiveWindowInfo } from '../../ipc';

export function WorkspaceBar() {
  const [workspaces, setWorkspaces] = useState<WorkspaceInfo[]>([]);
  const [activeId, setActiveId] = useState<number>(1);
  const [activeWindow, setActiveWindow] = useState<ActiveWindowInfo | null>(null);
  const [hyprlandRunning, setHyprlandRunning] = useState(false);

  const refresh = () => {
    getWorkspaces()
      .then((ws) => {
        setWorkspaces(ws);
        setHyprlandRunning(true);
      })
      .catch(() => setHyprlandRunning(false));
    getActiveWindow()
      .then((w) => {
        setActiveWindow(w);
        if (w) setActiveId(w.workspaceId);
      })
      .catch(() => {});
  };

  useEffect(() => {
    refresh();
    const interval = window.setInterval(refresh, 3000);

    let unlisten: (() => void) | undefined;
    onHyprlandEvent((ev) => {
      if (['workspace', 'activewindow', 'openwindow', 'closewindow', 'createworkspace', 'destroyworkspace'].includes(ev.event)) {
        refresh();
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    return () => {
      window.clearInterval(interval);
      unlisten?.();
    };
  }, []);

  if (!hyprlandRunning) {
    return (
      <div class="workspace-bar not-running">
        <span class="ws-hint">Hyprland not detected</span>
      </div>
    );
  }

  return (
    <div class="workspace-bar">
      <div class="ws-list">
        {workspaces.map((ws) => (
          <button
            key={ws.id}
            class={`ws-btn ${ws.id === activeId ? 'active' : ''} ${ws.windows > 0 ? 'occupied' : ''}`}
            onClick={() => switchWorkspace(ws.id).catch(() => {})}
            title={`Workspace ${ws.name} (${ws.windows} windows)`}
          >
            {ws.name}
            {ws.windows > 0 && <span class="ws-count">{ws.windows}</span>}
          </button>
        ))}
      </div>
      {activeWindow && (
        <div class="ws-active-window" title={activeWindow.title}>
          <span class="ws-window-class">{activeWindow.class}</span>
          <span class="ws-window-title">{activeWindow.title}</span>
        </div>
      )}
    </div>
  );
}
