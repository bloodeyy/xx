// ============================================
// RPC Session (Corrected for discord_social_rpc 0.1.2)
// ============================================

use discord_social_rpc::{
    Activity, ActivityType, Assets, DiscordRpcClient, DiscordSocialRpc, Timestamps,
};
use serde_json::Value;

pub struct RpcSession {
    _factory: DiscordSocialRpc,
    client: DiscordRpcClient,
    user_name: Option<String>,
    connected: bool,
}

impl RpcSession {
    pub fn new(_user_id: &str, token: &str) -> Result<Self, String> {
        let app_id = std::env::var("DISCORD_CLIENT_ID")
            .map_err(|_| "DISCORD_CLIENT_ID env var not set".to_string())?;

        let factory = DiscordSocialRpc::new(&app_id)
            .map_err(|e| format!("Factory error: {}", e))?;

        let client = factory
            .create_new_client(token)
            .map_err(|e| format!("Client create error: {}", e))?;

        Ok(Self {
            _factory: factory,
            client,
            user_name: None,
            connected: false,
        })
    }

    pub fn user_name(&self) -> Option<String> {
        self.user_name.clone()
    }

    pub fn set_activity_from_config(&mut self, config: &Value) -> Result<(), String> {
        let name = config
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Custom Activity")
            .to_string();

        let activity_type = config
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("PLAYING");

        // Correct builder API: .set_name(), .set_state(), etc.
        let mut activity = Activity::new().set_name(&name);

        if let Some(state) = config.get("state").and_then(|v| v.as_str()) {
            if !state.is_empty() {
                activity = activity.set_state(state);
            }
        }

        if let Some(details) = config.get("details").and_then(|v| v.as_str()) {
            if !details.is_empty() {
                activity = activity.set_details(details);
            }
        }

        // Map config type to ActivityType enum (Streaming not available in this crate)
        activity = activity.set_activity_type(match activity_type {
            "STREAMING" => ActivityType::Playing, // fallback
            "LISTENING" => ActivityType::Listening,
            "WATCHING" => ActivityType::Watching,
            "COMPETING" => ActivityType::Competing,
            _ => ActivityType::Playing,
        });

        // Images — correct API: .large_image(), .large_text(), etc. (without set_)
        let large_image = config
            .get("largeImage")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let small_image = config
            .get("smallImage")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !large_image.is_empty() || !small_image.is_empty() {
            let mut assets = Assets::new();

            if !large_image.is_empty() {
                assets = assets.large_image(large_image);
            }

            if let Some(large_text) = config.get("largeText").and_then(|v| v.as_str()) {
                if !large_text.is_empty() {
                    assets = assets.large_text(large_text);
                }
            }

            if !small_image.is_empty() {
                assets = assets.small_image(small_image);
            }

            if let Some(small_text) = config.get("smallText").and_then(|v| v.as_str()) {
                if !small_text.is_empty() {
                    assets = assets.small_text(small_text);
                }
            }

            activity = activity.set_assets(assets);
        }

        // Timer — correct API: Timestamps::new().start(i64 milliseconds)
        if config
            .get("startTimestamp")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            activity = activity.timestamps(Timestamps::new().start(now_ms));
        }

        // Set activity
        self.client
            .set_activity(activity)
            .map_err(|e| format!("set_activity error: {}", e))?;

        if !self.connected {
            self.client
                .start_activity()
                .map_err(|e| format!("start_activity error: {}", e))?;

            self.connected = true;

            if let Some(name) = self.client.user_name() {
                self.user_name = Some(name);
            }
        }

        Ok(())
    }

    pub fn set_status(&mut self, _status: &str) -> Result<(), String> {
        // Presence status changes ke liye alag method
        // Abhi ke liye placeholder
        Ok(())
    }

    pub fn clear_activity(&mut self) -> Result<(), String> {
        if self.connected {
            self.client
                .stop_activity()
                .map_err(|e| format!("stop_activity error: {}", e))?;
            self.connected = false;
        }
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.clear_activity()
    }
}
