use super::state::UiState;

pub struct TopBarText {
    pub left: String,
    pub center: String,
    pub right: String,
}

pub fn compose(state: &UiState) -> TopBarText {
    let workspace = state.active_tab.saturating_add(1);
    let left = format!("{} | ws:{}", state.hostname, workspace);
    let center = "eDEX-DE".to_string();
    let network_icon =
        if state.status.vpn_active || state.status.tor_active || state.status.tailscale_active {
            "◉"
        } else {
            "○"
        };
    let right = format!("{} {} {}", network_icon, state.date, state.clock);

    TopBarText {
        left,
        center,
        right,
    }
}
