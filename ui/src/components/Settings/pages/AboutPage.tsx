import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';

interface SystemOverview {
  hostname?: string;
  os?: string;
  kernel?: string;
  uptime?: string;
  cpu?: string;
  memory?: string;
}

const VERSION = '1.2.0';

export function AboutPage() {
  const [overview, setOverview] = useState<SystemOverview>({});

  useEffect(() => {
    invoke<any>('get_system_overview').then((data) => {
      setOverview({
        hostname: data.hostname,
        os: data.os,
        kernel: data.kernel,
        uptime: data.uptime,
        cpu: data.cpu,
        memory: data.memory,
      });
    }).catch(() => {});
  }, []);

  return (
    <div class="settings-page">
      <h2 class="settings-page-title">About</h2>

      <div class="settings-section">
        <h3 class="settings-section-title">eDEX-DE</h3>
        <div class="about-logo">eDEX-DE</div>
        <div class="settings-info-block">
          <span class="settings-info-key">Version</span>
          <span class="settings-info-val">v{VERSION}</span>
        </div>
        <div class="settings-info-block">
          <span class="settings-info-key">Built with</span>
          <span class="settings-info-val">Tauri · Preact · Hyprland</span>
        </div>
        <div class="settings-info-block">
          <span class="settings-info-key">Repository</span>
          <span class="settings-info-val">
            <a
              href="https://github.com/eDEX-OS/eDEX-DE"
              target="_blank"
              rel="noopener noreferrer"
              class="settings-link"
            >
              github.com/eDEX-OS/eDEX-DE
            </a>
          </span>
        </div>
      </div>

      {Object.keys(overview).length > 0 && (
        <div class="settings-section">
          <h3 class="settings-section-title">System</h3>
          {overview.hostname && (
            <div class="settings-info-block">
              <span class="settings-info-key">Hostname</span>
              <span class="settings-info-val">{overview.hostname}</span>
            </div>
          )}
          {overview.os && (
            <div class="settings-info-block">
              <span class="settings-info-key">OS</span>
              <span class="settings-info-val">{overview.os}</span>
            </div>
          )}
          {overview.kernel && (
            <div class="settings-info-block">
              <span class="settings-info-key">Kernel</span>
              <span class="settings-info-val">{overview.kernel}</span>
            </div>
          )}
          {overview.uptime && (
            <div class="settings-info-block">
              <span class="settings-info-key">Uptime</span>
              <span class="settings-info-val">{overview.uptime}</span>
            </div>
          )}
          {overview.cpu && (
            <div class="settings-info-block">
              <span class="settings-info-key">CPU</span>
              <span class="settings-info-val">{overview.cpu}</span>
            </div>
          )}
          {overview.memory && (
            <div class="settings-info-block">
              <span class="settings-info-key">Memory</span>
              <span class="settings-info-val">{overview.memory}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
