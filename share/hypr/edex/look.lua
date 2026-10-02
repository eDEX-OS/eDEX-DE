-- Look and feel: the eDEX canvas draws the chrome, Hyprland only frames app windows.
hl.config({
    general = {
        gaps_in = 4,
        gaps_out = 8,
        border_size = 2,
        col = {
            active_border = "rgba(00d4ffee)",
            inactive_border = "rgba(00d4ff33)",
        },
        resize_on_border = true,
        allow_tearing = false,
        layout = "dwindle",
    },
    decoration = {
        rounding = 0,
        active_opacity = 1.0,
        inactive_opacity = 1.0,
        shadow = { enabled = false },
        blur = { enabled = false },
    },
    animations = { enabled = true },
    dwindle = { preserve_split = true },
    master = { new_status = "master" },
    misc = {
        disable_hyprland_logo = true,
        disable_splash_rendering = true,
        force_default_wallpaper = 0,
        background_color = 0x0a0e1a,
        focus_on_activate = true,
        middle_click_paste = true,
        key_press_enables_dpms = true,
        mouse_move_enables_dpms = true,
        -- edex-session sets XDG_CURRENT_DESKTOP=eDEX-DE:Hyprland on purpose (portals and apps key
        -- off it); without this Hyprland shows a "managed externally" warning at every login.
        disable_xdg_env_checks = true,
    },
    xwayland = { enabled = true, force_zero_scaling = true },
    cursor = { inactive_timeout = 5, hide_on_key_press = false },
    binds = { workspace_back_and_forth = true },
})

hl.curve("edexOut", { type = "bezier", points = { { 0.16, 1.0 }, { 0.3, 1.0 } } })
hl.curve("edexLinear", { type = "bezier", points = { { 0.0, 0.0 }, { 1.0, 1.0 } } })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "windows", enabled = true, speed = 4, bezier = "edexOut" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 4, bezier = "edexOut", style = "popin 90%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 3, bezier = "edexLinear", style = "popin 90%" })
hl.animation({ leaf = "border", enabled = true, speed = 6, bezier = "edexOut" })
hl.animation({ leaf = "fade", enabled = true, speed = 3, bezier = "edexLinear" })
hl.animation({ leaf = "layers", enabled = false, speed = 1, bezier = "edexLinear" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 3, bezier = "edexOut", style = "fade" })
