# Contributing to eDEX-DE

## Dev Environment Setup

### Prerequisites

- **Rust** stable: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** 20+ LTS: https://nodejs.org
- **Tauri CLI**: installed via npm (`@tauri-apps/cli` in devDependencies)
- **System libs** (Debian/Ubuntu):
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev                            libasound2-dev libayatana-appindicator3-dev pkg-config
  ```
- **System libs** (Fedora):
  ```bash
  sudo dnf install webkit2gtk4.1-devel gtk3-devel openssl-devel                            alsa-lib-devel libayatana-appindicator-gtk3-devel
  ```

### Setup

```bash
git clone https://github.com/eDEX-OS/eDEX-DE.git
cd eDEX-DE
npm install
```

### Build Commands

| Command | Description |
|---------|-------------|
| `npm run tauri -- dev` | Start dev server with hot reload |
| `npm run build` | Build frontend only |
| `npm run typecheck` | TypeScript type check |
| `cargo check -p edex-de` | Rust compile check |
| `cargo test -p edex-de` | Run Rust tests |
| `npm run tauri -- build` | Full production build |
| `npm run tauri -- build --no-bundle` | Build binary without packaging |

---

## Branch Naming

| Prefix | Use for |
|--------|---------|
| `feature/` | New features |
| `fix/` | Bug fixes |
| `docs/` | Documentation changes |
| `chore/` | Build, CI, tooling |
| `refactor/` | Code refactoring without behavior change |

Example: `feature/lock-screen`, `fix/terminal-resize`, `docs/hyprland-guide`

---

## Commit Style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add lock screen with fprintd fingerprint prompt
fix: terminal resize message format for large terminals
docs: add Hyprland keybind examples to README
chore: bump Tauri to v2.1
refactor: split sysinfo command into per-subsystem modules
```

---

## PR Checklist

Before submitting a pull request:

- [ ] `npm run typecheck` passes
- [ ] `cargo check -p edex-de` passes
- [ ] No new `console.error` or `unwrap()` without comment
- [ ] CSS changes appended to `ui/src/styles/main.css` (never replace)
- [ ] New Tauri commands registered in `src-tauri/src/lib.rs` `generate_handler!`
- [ ] New command modules exported in `src-tauri/src/commands/mod.rs`
- [ ] New IPC wrappers added to `ui/src/ipc/index.ts`
- [ ] Version bumped in all 4 places if releasing: `Cargo.toml`, `tauri.conf.json`, `package.json`, `BootScreen.tsx`

---

## Code Style

- **Rust**: `cargo fmt` before committing; `cargo clippy` warnings addressed
- **TypeScript**: Preact functional components with hooks; no class components
- **CSS**: Append to `main.css`; use CSS custom properties (`var(--color-accent)`)
- **No `document.getElementById`** — use Preact refs or state
- **No `any` types** in TypeScript unless absolutely unavoidable

---

## Architecture Notes

- All Rust–JS communication goes through `invoke()` / `emit()` / `listen()` (Tauri IPC)
- System integrations use CLI tools (`pactl`, `nmcli`, `systemctl`) to avoid native lib build complexity
- New Tauri commands: add to `src-tauri/src/commands/<module>.rs`, export from `mod.rs`, register in `lib.rs`
- New UI components: add to `ui/src/components/<Name>/index.ts` + `<Name>.tsx`, export from `ui/src/components/index.ts`
