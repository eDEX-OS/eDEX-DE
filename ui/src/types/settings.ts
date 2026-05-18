export interface Settings {
  shell: string;
  shellArgs: string;
  cwd: string;
  keyboard: string;
  theme: string;
  termFontSize: number;
  shellOpacity: number;
  audio: boolean;
  audioVolume: number;
  disableFeedbackAudio: boolean;
  clockHours: number;
  pingAddr: string;
  port: number;
  nointro: boolean;
  nocursor: boolean;
  forceFullscreen: boolean;
  allowWindowed: boolean;
  excludeThreadsFromToplist: boolean;
  hideDotfiles: boolean;
  fsListView: boolean;
  experimentalGlobeFeatures: boolean;
  experimentalFeatures: boolean;
  monitor?: number;
  env?: Record<string, string>;
}

export interface Shortcut {
  type: 'app' | 'xterm';
  trigger: string;
  action: string;
  enabled: boolean;
  linebreak?: boolean;
}

export const defaultSettings: Settings = {
  shell: 'bash',
  shellArgs: '',
  cwd: '~',
  keyboard: 'en-US',
  theme: 'tron',
  termFontSize: 15,
  shellOpacity: 1.0,
  audio: true,
  audioVolume: 1.0,
  disableFeedbackAudio: false,
  clockHours: 24,
  pingAddr: '1.1.1.1',
  port: 3000,
  nointro: false,
  nocursor: false,
  forceFullscreen: true,
  allowWindowed: false,
  excludeThreadsFromToplist: true,
  hideDotfiles: false,
  fsListView: false,
  experimentalGlobeFeatures: false,
  experimentalFeatures: false,
};
