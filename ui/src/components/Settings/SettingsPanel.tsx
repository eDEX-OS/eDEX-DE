import { useState } from 'preact/hooks';
import {
  AppearancePage, DisplayPage, InputPage, AudioPage, NetworkPage,
  BluetoothPage, PowerPage, SecurityPrivacyPage, UsersPage,
  NotificationsPage, SystemServicesPage, HyprlandPage, TerminalPage, AboutPage,
} from './pages';

type PageId =
  | 'appearance' | 'display' | 'input' | 'audio' | 'network'
  | 'bluetooth' | 'power' | 'security' | 'users' | 'notifications'
  | 'services' | 'hyprland' | 'terminal' | 'about';

interface NavItem {
  id: PageId;
  label: string;
  icon: string;
}

const NAV_ITEMS: NavItem[] = [
  { id: 'appearance', label: 'Appearance', icon: '🎨' },
  { id: 'display', label: 'Display', icon: '🖥' },
  { id: 'input', label: 'Input', icon: '⌨' },
  { id: 'audio', label: 'Audio', icon: '🔊' },
  { id: 'network', label: 'Network', icon: '📡' },
  { id: 'bluetooth', label: 'Bluetooth', icon: '🔷' },
  { id: 'power', label: 'Power', icon: '⚡' },
  { id: 'security', label: 'Security & Privacy', icon: '🔒' },
  { id: 'users', label: 'Users', icon: '👤' },
  { id: 'notifications', label: 'Notifications', icon: '🔔' },
  { id: 'services', label: 'System Services', icon: '⚙' },
  { id: 'hyprland', label: 'Hyprland', icon: '🪟' },
  { id: 'terminal', label: 'Terminal', icon: '💻' },
  { id: 'about', label: 'About', icon: 'ℹ' },
];

interface SettingsPanelProps {
  onClose: () => void;
  initialPage?: PageId;
}

export function SettingsPanel({ onClose, initialPage = 'appearance' }: SettingsPanelProps) {
  const [activePage, setActivePage] = useState<PageId>(initialPage);

  const renderPage = () => {
    switch (activePage) {
      case 'appearance': return <AppearancePage />;
      case 'display': return <DisplayPage />;
      case 'input': return <InputPage />;
      case 'audio': return <AudioPage />;
      case 'network': return <NetworkPage />;
      case 'bluetooth': return <BluetoothPage />;
      case 'power': return <PowerPage />;
      case 'security': return <SecurityPrivacyPage />;
      case 'users': return <UsersPage />;
      case 'notifications': return <NotificationsPage />;
      case 'services': return <SystemServicesPage />;
      case 'hyprland': return <HyprlandPage />;
      case 'terminal': return <TerminalPage />;
      case 'about': return <AboutPage />;
      default: return null;
    }
  };

  return (
    <div class="settings-overlay" onClick={onClose}>
      <div class="settings-panel" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div class="settings-panel-header">
          <span class="settings-panel-title">SYSTEM SETTINGS</span>
          <button class="settings-panel-close" onClick={onClose}>×</button>
        </div>

        <div class="settings-panel-body">
          {/* Sidebar nav */}
          <nav class="settings-panel-nav">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.id}
                class={`settings-nav-item${activePage === item.id ? ' active' : ''}`}
                onClick={() => setActivePage(item.id)}
              >
                <span class="settings-nav-icon">{item.icon}</span>
                <span class="settings-nav-label">{item.label}</span>
              </button>
            ))}
          </nav>

          {/* Page content */}
          <div class="settings-panel-content">
            {renderPage()}
          </div>
        </div>
      </div>
    </div>
  );
}
