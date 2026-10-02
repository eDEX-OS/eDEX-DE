-- Keybindings. Shell overlays are toggled through `edex-de ipc` so they work even when an
-- application window has focus.
local mod = "SUPER"
local ipc = function(args)
    return hl.dsp.exec_cmd("edex-de ipc " .. args)
end

-- Shell overlays. Tapping the Super (Windows) key on its own opens the launcher; it fires on
-- release and not when Super is used in a combination.
hl.bind(mod .. " + Super_L", ipc("toggle launcher"), { release = true })
hl.bind(mod .. " + space", ipc("toggle launcher"))
hl.bind(mod .. " + comma", ipc("toggle settings"))
hl.bind(mod .. " + P", ipc("toggle privacy"))
hl.bind(mod .. " + N", ipc("toggle notifications"))
hl.bind(mod .. " + Escape", ipc("toggle power"))
hl.bind(mod .. " + Return", ipc("focus terminal"))
hl.bind(mod .. " + F1", ipc("focus filesystem"))

-- Applications
hl.bind(mod .. " + SHIFT + Return", hl.dsp.exec_cmd("kitty"))
-- Files: ranger in a new eDEX terminal tab, in the file panel's directory.
hl.bind(mod .. " + E", ipc("files"))
hl.bind(mod .. " + B", hl.dsp.exec_cmd("xdg-open https://"))

-- Window management
hl.bind(mod .. " + Q", hl.dsp.window.close())
hl.bind(mod .. " + SHIFT + Q", hl.dsp.window.kill())
-- Maximize (the □ button): the window gets the full width; the shell hides its side panels
-- while it is maximized and keeps the tab strip with the window controls visible.
hl.bind(mod .. " + F", hl.dsp.window.fullscreen({ action = "toggle", mode = "maximized" }))
-- True fullscreen over everything (games, video).
hl.bind(mod .. " + SHIFT + F", hl.dsp.window.fullscreen({ action = "toggle", mode = "fullscreen" }))
-- Minimize (the ↓ button): the window becomes a tab in the centre panel; click it to restore.
hl.bind(mod .. " + M", hl.dsp.window.move({ workspace = "special:minimized", follow = false }))
-- Hide/show the file and system side panels for every app.
hl.bind(mod .. " + CTRL + F", ipc("action side-panels"))
hl.bind(mod .. " + V", hl.dsp.window.float({ action = "toggle" }))
hl.bind(mod .. " + C", hl.dsp.window.center())
hl.bind(mod .. " + T", hl.dsp.window.pseudo())
hl.bind(mod .. " + J", hl.dsp.layout("togglesplit"))
hl.bind(mod .. " + G", hl.dsp.group.toggle())
hl.bind(mod .. " + Tab", hl.dsp.window.cycle_next())
hl.bind(mod .. " + SHIFT + Tab", hl.dsp.window.cycle_next({ prev = true }))

for key, dir in pairs({ H = "left", L = "right", K = "up", J = "down" }) do
    if key ~= "J" then
        hl.bind(mod .. " + " .. key, hl.dsp.focus({ direction = dir }))
    end
    hl.bind(mod .. " + SHIFT + " .. key, hl.dsp.window.move({ direction = dir }))
    hl.bind(mod .. " + CTRL + " .. key, hl.dsp.window.resize({ x = (dir == "left" and -40) or (dir == "right" and 40) or 0, y = (dir == "up" and -40) or (dir == "down" and 40) or 0, relative = true }))
end
for key, dir in pairs({ left = "left", right = "right", up = "up", down = "down" }) do
    hl.bind(mod .. " + " .. key, hl.dsp.focus({ direction = dir }))
    hl.bind(mod .. " + SHIFT + " .. key, hl.dsp.window.move({ direction = dir }))
end

-- Workspaces
for i = 1, 10 do
    local key = i % 10
    hl.bind(mod .. " + " .. key, hl.dsp.focus({ workspace = i }))
    hl.bind(mod .. " + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
    hl.bind(mod .. " + CTRL + " .. key, hl.dsp.window.move({ workspace = i, follow = true }))
end
hl.bind(mod .. " + S", hl.dsp.workspace.toggle_special("scratch"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special:scratch" }))
hl.bind(mod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mod .. " + mouse_up", hl.dsp.focus({ workspace = "e-1" }))
hl.bind(mod .. " + bracketright", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mod .. " + bracketleft", hl.dsp.focus({ workspace = "e-1" }))

-- Mouse
hl.bind(mod .. " + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind(mod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- Session
hl.bind(mod .. " + SHIFT + L", hl.dsp.exec_cmd("loginctl lock-session"))
hl.bind(mod .. " + SHIFT + R", hl.dsp.exec_cmd("hyprctl reload && edex-de ipc reload"))

-- Screenshots (grim + slurp → clipboard and ~/Pictures/Screenshots)
local shot_dir = os.getenv("HOME") .. "/Pictures/Screenshots"
hl.bind("Print", hl.dsp.exec_cmd("mkdir -p " .. shot_dir .. " && grim " .. shot_dir .. "/$(date +%Y%m%d-%H%M%S).png && edex-de ipc notify Screenshot saved"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("grim -g \"$(slurp)\" - | wl-copy && edex-de ipc notify Screenshot copied to clipboard"))
hl.bind(mod .. " + SHIFT + Print", hl.dsp.exec_cmd("mkdir -p " .. shot_dir .. " && grim -g \"$(slurp)\" " .. shot_dir .. "/$(date +%Y%m%d-%H%M%S).png"))

-- Media keys route through the shell so the OSD is shown.
hl.bind("XF86AudioRaiseVolume", ipc("audio volume +5"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", ipc("audio volume -5"), { locked = true, repeating = true })
hl.bind("XF86AudioMute", ipc("audio mute"), { locked = true })
hl.bind("XF86AudioMicMute", ipc("audio mic-mute"), { locked = true })
hl.bind("XF86MonBrightnessUp", ipc("brightness +5"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", ipc("brightness -5"), { locked = true, repeating = true })
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"), { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"), { locked = true })
hl.bind("XF86PowerOff", ipc("toggle power"), { locked = true })
