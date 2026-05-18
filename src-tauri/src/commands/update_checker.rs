use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub has_update: bool,
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
}

#[tauri::command]
pub async fn check_for_update(current_version: String) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("edex-ui-hyprland/update-checker")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response: serde_json::Value = client
        .get("https://api.github.com/repos/0xnullsect0r/edex-ui-hyprland/releases/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let latest = response["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let release_url = response["html_url"].as_str().unwrap_or("").to_string();
    let has_update = !latest.is_empty() && latest != current_version.trim_start_matches('v');

    Ok(UpdateInfo {
        has_update,
        latest_version: latest,
        current_version,
        release_url,
    })
}
