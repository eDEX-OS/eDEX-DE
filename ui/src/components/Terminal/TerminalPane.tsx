import { useEffect, useRef, useState } from 'preact/hooks';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';
import { useSettings } from '../../context';
import { closeTerminal, spawnTerminal } from '../../ipc';

interface TerminalPaneProps {
  port: number;
  active: boolean;
}

export function TerminalPane({ port, active }: TerminalPaneProps) {
  const { settings } = useSettings();
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      fontSize: settings.termFontSize,
      fontFamily: "'Share Tech Mono', 'Fira Code', monospace",
      theme: {
        background: '#0a0c10',
        foreground: '#e0e8f0',
        cursor: '#00e5ff',
        cursorAccent: '#0a0c10',
        selectionBackground: 'rgba(0, 229, 255, 0.25)',
        black: '#0a0c10',
        red: '#ff3366',
        green: '#00ff88',
        yellow: '#ffcc00',
        blue: '#00e5ff',
        magenta: '#c97bff',
        cyan: '#00e5ff',
        white: '#e0e8f0',
        brightBlack: '#3d5166',
        brightRed: '#ff6688',
        brightGreen: '#33ff99',
        brightYellow: '#ffdd33',
        brightBlue: '#33eeff',
        brightMagenta: '#dd99ff',
        brightCyan: '#33eeff',
        brightWhite: '#ffffff',
      },
      cursorBlink: !settings.nocursor,
      scrollback: 5000,
      allowTransparency: true,
    });

    const fit = new FitAddon();
    const links = new WebLinksAddon();
    term.loadAddon(fit);
    term.loadAddon(links);
    term.open(containerRef.current);
    fit.fit();

    termRef.current = term;
    fitRef.current = fit;

    const shell = settings.shell || 'bash';
    const cwd = settings.cwd || '~';
    const env: [string, string][] = settings.env ? Object.entries(settings.env) : [];

    spawnTerminal(port, shell, cwd, env)
      .then(() => {
        const ws = new WebSocket(`ws://127.0.0.1:${port}`);
        ws.binaryType = 'arraybuffer';
        wsRef.current = ws;

        ws.onopen = () => {
          setConnected(true);
          const dims = fit.proposeDimensions();
          if (dims) {
            const cols = String(dims.cols).padStart(3, '0');
            const rows = String(dims.rows).padStart(3, '0');
            ws.send(`Resize${cols}${rows}`);
          }
        };

        ws.onmessage = (ev) => {
          if (ev.data instanceof ArrayBuffer) {
            term.write(new Uint8Array(ev.data));
          } else {
            term.write(ev.data as string);
          }
        };

        ws.onclose = () => setConnected(false);
        ws.onerror = () => setError('WebSocket error');

        term.onData((data) => {
          if (ws.readyState === WebSocket.OPEN) ws.send(data);
        });

        term.onResize(({ cols, rows }) => {
          if (ws.readyState === WebSocket.OPEN) {
            const c = String(cols).padStart(3, '0');
            const r = String(rows).padStart(3, '0');
            ws.send(`Resize${c}${r}`);
          }
        });
      })
      .catch((e) => setError(String(e)));

    const ro = new ResizeObserver(() => fit.fit());
    ro.observe(containerRef.current);

    return () => {
      ro.disconnect();
      wsRef.current?.close();
      term.dispose();
      closeTerminal(port).catch(() => {});
    };
  }, [port]);

  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.fontSize = settings.termFontSize;
      fitRef.current?.fit();
    }
  }, [settings.termFontSize]);

  if (error) {
    return (
      <div class="terminal-error">
        <span>Terminal error: {error}</span>
      </div>
    );
  }

  return (
    <div class={`terminal-pane ${active ? 'active' : ''}`}>
      {!connected && <div class="terminal-connecting">Connecting...</div>}
      <div ref={containerRef} class="xterm-container" style={{ opacity: settings.shellOpacity }} />
    </div>
  );
}
