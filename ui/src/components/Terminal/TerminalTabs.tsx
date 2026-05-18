import type { TerminalTab } from '../../types';

interface TerminalTabsProps {
  tabs: TerminalTab[];
  activeId: number;
  onSelect: (id: number) => void;
  onClose: (id: number) => void;
  onNew: () => void;
}

export function TerminalTabs({ tabs, activeId, onSelect, onClose, onNew }: TerminalTabsProps) {
  return (
    <div class="terminal-tabs">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          class={`terminal-tab ${tab.id === activeId ? 'active' : ''}`}
          onClick={() => onSelect(tab.id)}
        >
          <span class="tab-title">{tab.title}</span>
          <button
            class="tab-close"
            onClick={(e) => {
              e.stopPropagation();
              onClose(tab.id);
            }}
            aria-label="Close tab"
          >
            ×
          </button>
        </div>
      ))}
      <button class="tab-new" onClick={onNew} title="New tab (Ctrl+Shift+T)">
        +
      </button>
    </div>
  );
}
