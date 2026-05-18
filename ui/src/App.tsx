import { useState, useEffect } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";

export function App() {
  const [greeting, setGreeting] = useState<string>("Initializing eDEX-UI...");

  useEffect(() => {
    invoke<string>("greet", { name: "Hyprland" }).then(setGreeting).catch(console.error);
  }, []);

  return (
    <div class="edex-root">
      <div class="boot-screen">
        <pre class="ascii-logo">{`
 _____  _______  __       __      __  __
|  ___|  ___   \\ \\ \\     / /     /  |/  |
| |__  | |  | |  \\ \\   / /     / /|  / |
|  __| | |  | |   \\ \\ / /     / / | / / 
| |___ | |__| |    \\   /     / /  |/ /  
|_____|\\______/      \\_/     /_/      /   
`}</pre>
        <p class="status">{greeting}</p>
        <p class="version">eDEX-UI Hyprland v0.1.0 — Tauri + Rust + Preact</p>
      </div>
    </div>
  );
}
