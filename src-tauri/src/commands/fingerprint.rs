use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FprintdStatus {
    pub available: bool,
    pub has_enrolled_fingers: bool,
    pub device_name: Option<String>,
}

fn current_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| "root".into())
}

#[tauri::command]
pub fn fprintd_status() -> FprintdStatus {
    let user = current_user();
    let out = Command::new("fprintd-list").arg(&user).output();

    let Ok(output) = out else {
        return FprintdStatus {
            available: false,
            has_enrolled_fingers: false,
            device_name: None,
        };
    };

    if !output.status.success() {
        return FprintdStatus {
            available: false,
            has_enrolled_fingers: false,
            device_name: None,
        };
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let has_enrolled_fingers = !text.trim().is_empty() && !text.to_lowercase().contains("no fingers enrolled");
    let device_name = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty());

    FprintdStatus {
        available: true,
        has_enrolled_fingers,
        device_name,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn fprintd_verify(app_handle: tauri::AppHandle) -> Result<VerifyResult, String> {
    use tauri::Emitter;

    let _ = app_handle.emit("fprintd-status", "swipe");

    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        tokio::task::spawn_blocking(|| {
            Command::new("fprintd-verify")
                .arg("-f")
                .arg("any")
                .output()
        }),
    )
    .await;

    match result {
        Ok(Ok(Ok(output))) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
            let success = stdout.contains("verify-match") || stdout.contains("verify matched");
            let _ = app_handle.emit("fprintd-status", if success { "match" } else { "no-match" });
            Ok(VerifyResult {
                success,
                message: if success {
                    "Fingerprint verified".to_string()
                } else {
                    "Fingerprint did not match".to_string()
                },
            })
        }
        Ok(Ok(Err(e))) => Err(e.to_string()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => {
            let _ = app_handle.emit("fprintd-status", "timeout");
            Err("Fingerprint verification timed out".to_string())
        }
    }
}
