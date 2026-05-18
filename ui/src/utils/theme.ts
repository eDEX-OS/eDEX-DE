export interface ThemeColors {
  background: string;
  foreground: string;
  accent: string;
  secondary: string;
  error: string;
  warning: string;
  success: string;
  border: string;
}

const themes: Record<string, ThemeColors> = {
  tron: {
    background: '#0a0c10',
    foreground: '#e0e8f0',
    accent: '#00e5ff',
    secondary: '#00ff88',
    error: '#ff3366',
    warning: '#ffcc00',
    success: '#00ff88',
    border: '#1a2535',
  },
  default: {
    background: '#000000',
    foreground: '#ffffff',
    accent: '#00e5ff',
    secondary: '#00ff88',
    error: '#ff0000',
    warning: '#ffff00',
    success: '#00ff00',
    border: '#333333',
  },
};

export function getTheme(name: string): ThemeColors {
  return themes[name] ?? themes.tron;
}

export function applyThemeToCssVars(name: string): void {
  const theme = getTheme(name);
  const root = document.documentElement;
  root.style.setProperty('--color-bg', theme.background);
  root.style.setProperty('--color-fg', theme.foreground);
  root.style.setProperty('--color-accent', theme.accent);
  root.style.setProperty('--color-secondary', theme.secondary);
  root.style.setProperty('--color-error', theme.error);
  root.style.setProperty('--color-warning', theme.warning);
  root.style.setProperty('--color-success', theme.success);
  root.style.setProperty('--color-border', theme.border);
}
