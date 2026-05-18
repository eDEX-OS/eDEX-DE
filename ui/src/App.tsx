import { useCallback, useEffect, useState } from 'preact/hooks';
import { SettingsProvider, SysinfoProvider, useSettings } from './context';
import { BootScreen, TopBar, StatusBar, Placeholder } from './components';
import { applyThemeToCssVars } from './utils';
import './styles/main.css';

function AppShell() {
  const { settings, loaded } = useSettings();
  const [booted, setBooted] = useState(false);
  const handleBootComplete = useCallback(() => setBooted(true), []);

  useEffect(() => {
    if (loaded) {
      applyThemeToCssVars(settings.theme);
    }
  }, [loaded, settings.theme]);

  if (!loaded || !booted) {
    return <BootScreen onComplete={handleBootComplete} skip={!loaded ? false : settings.nointro} />;
  }

  return (
    <div class="app-layout">
      <TopBar />
      <div class="app-body">
        <div class="panel panel-left">
          <Placeholder label="Filesystem" description="Phase 4" />
        </div>
        <div class="panel panel-center">
          <Placeholder label="Terminal" description="Phase 4" />
        </div>
        <div class="panel panel-right">
          <Placeholder label="Sysinfo" description="Phase 4" />
        </div>
      </div>
      <StatusBar />
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
