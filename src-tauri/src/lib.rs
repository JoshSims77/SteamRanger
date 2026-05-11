use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Deserialize)]
struct GetAchievementsRequest {
    appid: String,
    api_key: String,
}

#[derive(Serialize)]
struct Achievement {
    id: String,
    name: String,
    description: String,
    icon: String,
    hidden: bool,
}

#[derive(Serialize)]
struct GameAchievements {
    appid: String,
    achievements: Vec<Achievement>,
}

#[tauri::command]
async fn get_achievements(req: GetAchievementsRequest) -> Result<GameAchievements, String> {
    let GetAchievementsRequest { appid, api_key } = req;

    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?key={}&appid={}",
        api_key, appid
    );

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    let achievements_json = resp["game"]["availableGameStats"]["achievements"]
        .as_array()
        .ok_or("No achievements found")?;

    let achievements = achievements_json
        .iter()
        .map(|a| Achievement {
            id: a["name"].as_str().unwrap_or("").to_string(),
            name: a["displayName"].as_str().unwrap_or("").to_string(),
            description: a["description"].as_str().unwrap_or("").to_string(),
            icon: a["icon"].as_str().unwrap_or("").to_string(),
            hidden: a["hidden"].as_bool().unwrap_or(false),
        })
        .collect();

    Ok(GameAchievements { appid, achievements })
}

#[tauri::command]
async fn download_icon(url: String, dest_path: String) -> Result<(), String> {
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read bytes: {}", e))?;

    let path = PathBuf::from(&dest_path);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    // Decode, resize to 128x128, and re-encode as PNG
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let resized = img.resize_exact(128, 128, image::imageops::FilterType::Lanczos3);

    resized.save(&path)
        .map_err(|e| format!("Failed to save image: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![get_achievements, download_icon])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}