import { createContext } from 'preact';
import type { ComponentChildren } from 'preact';
import { useContext, useEffect, useState } from 'preact/hooks';
import type { Settings, Shortcut } from '../types';
import { defaultSettings } from '../types';
import { loadSettings, saveSettings, loadShortcuts, saveShortcuts } from '../ipc';

interface SettingsContextValue {
  settings: Settings;
  shortcuts: Shortcut[];
  updateSettings: (patch: Partial<Settings>) => Promise<void>;
  updateShortcuts: (shortcuts: Shortcut[]) => Promise<void>;
  loaded: boolean;
}

const SettingsContext = createContext<SettingsContextValue>({
  settings: defaultSettings,
  shortcuts: [],
  updateSettings: async () => {},
  updateShortcuts: async () => {},
  loaded: false,
});

export function SettingsProvider({ children }: { children: ComponentChildren }) {
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [shortcuts, setShortcuts] = useState<Shortcut[]>([]);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    Promise.all([loadSettings(), loadShortcuts()])
      .then(([loadedSettings, loadedShortcuts]) => {
        setSettings(loadedSettings);
        setShortcuts(loadedShortcuts);
      })
      .catch((error) => {
        console.error(error);
      })
      .finally(() => {
        setLoaded(true);
      });
  }, []);

  const updateSettings = async (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    await saveSettings(next);
  };

  const updateShortcuts = async (nextShortcuts: Shortcut[]) => {
    setShortcuts(nextShortcuts);
    await saveShortcuts(nextShortcuts);
  };

  return (
    <SettingsContext.Provider value={{ settings, shortcuts, updateSettings, updateShortcuts, loaded }}>
      {children}
    </SettingsContext.Provider>
  );
}

export const useSettings = () => useContext(SettingsContext);
