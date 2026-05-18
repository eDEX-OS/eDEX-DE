import { useCallback, useState } from 'preact/hooks';
import { TerminalPane } from './TerminalPane';
import { TerminalTabs } from './TerminalTabs';
import type { TerminalTab } from '../../types';
import { useSettings } from '../../context';

let tabCounter = 1;

function makeTab(settings: { port: number }): TerminalTab {
  const id = tabCounter++;
  return { id, port: settings.port + id - 1, title: `Terminal ${id}` };
}

export function TerminalPanel() {
  const { settings } = useSettings();
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [makeTab({ port: settings.port })]);
  const [activeId, setActiveId] = useState<number>(1);

  const handleNew = useCallback(() => {
    const tab = makeTab({ port: settings.port });
    setTabs((prev) => [...prev, tab]);
    setActiveId(tab.id);
  }, [settings.port]);

  const handleClose = useCallback(
    (id: number) => {
      setTabs((prev) => {
        const next = prev.filter((t) => t.id !== id);
        if (next.length === 0) {
          const tab = makeTab({ port: settings.port });
          setActiveId(tab.id);
          return [tab];
        }
        if (activeId === id) setActiveId(next[next.length - 1].id);
        return next;
      });
    },
    [activeId, settings.port],
  );

  return (
    <div class="terminal-panel">
      <TerminalTabs
        tabs={tabs}
        activeId={activeId}
        onSelect={setActiveId}
        onClose={handleClose}
        onNew={handleNew}
      />
      <div class="terminal-body">
        {tabs.map((tab) => (
          <TerminalPane key={tab.id} port={tab.port} active={tab.id === activeId} />
        ))}
      </div>
    </div>
  );
}
