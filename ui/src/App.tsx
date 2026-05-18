import { useState, useCallback, useEffect } from 'preact/hooks';
import { SettingsProvider, SysinfoProvider, useSettings } from './context';
import {
  BootScreen, TopBar, StatusBar,
  TerminalPanel, FileList, SysinfoSidebar, SettingsModal, LauncherOverlay,
} from './components';
import { onToggleLauncher } from './ipc';
import { applyThemeToCssVars } from './utils';
import './styles/main.css';

function AppShell() {
  const { settings, loaded } = useSettings();
  const [booted, setBooted] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showLauncher, setShowLauncher] = useState(false);

  useEffect(() => {
    if (loaded) applyThemeToCssVars(settings.theme);
  }, [loaded, settings.theme]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onToggleLauncher(() => setShowLauncher((v) => !v))
      .then((fn) => { unlisten = fn; })
      .catch(console.error);
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'S') {
        e.preventDefault();
        setShowSettings((v) => !v);
      }
      if (e.altKey && e.code === 'Space') {
        e.preventDefault();
        setShowLauncher((v) => !v);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const handleBootComplete = useCallback(() => setBooted(true), []);

  if (!loaded || !booted) {
    return (
      <BootScreen
        onComplete={handleBootComplete}
        skip={!loaded ? false : settings.nointro}
      />
    );
  }

  return (
    <div class="app-layout">
      <TopBar />
      <div class="app-body">
        <div class="panel panel-left">
          <FileList initialPath={settings.cwd} />
        </div>
        <div class="panel panel-center">
          <TerminalPanel />
        </div>
        <div class="panel panel-right">
          <SysinfoSidebar />
        </div>
      </div>
      <StatusBar />
      {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
      {showLauncher && <LauncherOverlay onClose={() => setShowLauncher(false)} />}
    </div>
  );
}

export function App() {
  return (
    <SettingsProvider>
      <SysinfoProvider>
        <AppShell />
      </SysinfoProvider>
    </SettingsProvider>
  );
}
