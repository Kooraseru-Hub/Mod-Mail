use std::sync::Arc;
use reqwest::Client;
use serde_json::Value;

/// Which backend to use. Chosen at startup based on env vars.
#[derive(Clone)]
pub enum StorageBackend {
    File,
    Firestore(FirestoreClient),
}

impl StorageBackend {
    /// Build from environment. If `FIRESTORE_PROJECT_ID` is set, use Firestore.
    pub fn from_env() -> Self {
        match std::env::var("FIRESTORE_PROJECT_ID") {
            Ok(project_id) => {
                println!("[storage] Using Firestore backend (project: {})", project_id);
                StorageBackend::Firestore(FirestoreClient::new(project_id))
            }
            Err(_) => {
                println!("[storage] Using file backend");
                StorageBackend::File
            }
        }
    }

    pub async fn load_guild(&self, guild_id: u64) -> Option<Value> {
        match self {
            StorageBackend::File => load_file(guild_id),
            StorageBackend::Firestore(client) => client.get_guild(guild_id).await,
        }
    }

    pub async fn save_guild(&self, guild_id: u64, data: &Value) -> Result<(), String> {
        match self {
            StorageBackend::File => save_file(guild_id, data),
            StorageBackend::Firestore(client) => client.set_guild(guild_id, data).await,
        }
    }

    /// List all guild IDs that have configs stored (used by messaged to find user's guilds).
    pub async fn list_guild_ids(&self) -> Vec<u64> {
        match self {
            StorageBackend::File => list_file_guild_ids(),
            StorageBackend::Firestore(client) => client.list_guild_ids().await,
        }
    }
}

// ── Serenity TypeMap key ──────────────────────────────────────────────────────

pub struct StorageKey;

impl serenity::prelude::TypeMapKey for StorageKey {
    type Value = Arc<StorageBackend>;
}

/// Retrieve the storage backend from a Serenity Context.
pub async fn get(ctx: &serenity::prelude::Context) -> Arc<StorageBackend> {
    ctx.data
        .read()
        .await
        .get::<StorageKey>()
        .expect("StorageBackend missing from TypeMap")
        .clone()
}

// ── File backend ──────────────────────────────────────────────────────────────

fn guild_path(guild_id: u64) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("data/guilds/{}.json", guild_id))
}

fn load_file(guild_id: u64) -> Option<Value> {
    let path = guild_path(guild_id);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_file(guild_id: u64, data: &Value) -> Result<(), String> {
    let path = guild_path(guild_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    let text = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&path, text)
        .map_err(|e| format!("Failed to write file: {}", e))
}

fn list_file_guild_ids() -> Vec<u64> {
    let dir = std::path::Path::new("data/guilds");
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                return None;
            }
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok())
        })
        .collect()
}

// ── Firestore REST client ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FirestoreClient {
    project_id: String,
    http: Client,
}

impl FirestoreClient {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            http: Client::new(),
        }
    }

    /// Fetch an access token from the GCE metadata server (available on Cloud Run automatically).
    async fn access_token(&self) -> Option<String> {
        let resp = self
            .http
            .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .ok()?;
        let json: Value = resp.json().await.ok()?;
        json.get("access_token")?.as_str().map(str::to_owned)
    }

    fn doc_url(&self, guild_id: u64) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/guilds/{}",
            self.project_id, guild_id
        )
    }

    fn list_url(&self) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/guilds",
            self.project_id
        )
    }

    pub async fn get_guild(&self, guild_id: u64) -> Option<Value> {
        let token = self.access_token().await?;
        let resp = self
            .http
            .get(self.doc_url(guild_id))
            .bearer_auth(&token)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let doc: Value = resp.json().await.ok()?;
        firestore_doc_to_value(&doc)
    }

    pub async fn set_guild(&self, guild_id: u64, data: &Value) -> Result<(), String> {
        let token = self
            .access_token()
            .await
            .ok_or("Failed to get access token")?;

        let doc = value_to_firestore_doc(data);

        let resp = self
            .http
            .patch(self.doc_url(guild_id))
            .bearer_auth(&token)
            .json(&doc)
            .send()
            .await
            .map_err(|e| format!("Firestore request failed: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Firestore error {}: {}", status, body))
        }
    }

    pub async fn list_guild_ids(&self) -> Vec<u64> {
        let token = match self.access_token().await {
            Some(t) => t,
            None => return vec![],
        };

        let resp = match self
            .http
            .get(self.list_url())
            .bearer_auth(&token)
            .query(&[("pageSize", "1000")])
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let json: Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        json.get("documents")
            .and_then(|d| d.as_array())
            .map(|docs| {
                docs.iter()
                    .filter_map(|doc| {
                        // name is like "projects/.../documents/guilds/GUILD_ID"
                        doc.get("name")?
                            .as_str()?
                            .split('/')
                            .last()?
                            .parse::<u64>()
                            .ok()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ── Firestore ↔ JSON conversion ───────────────────────────────────────────────
//
// Firestore REST API uses a typed value format. We store the entire GuildConfig
// as a single JSON string in a "data" field to keep conversion simple and
// avoid mapping every field to Firestore types.

fn value_to_firestore_doc(value: &Value) -> Value {
    let json_str = value.to_string();
    serde_json::json!({
        "fields": {
            "data": {
                "stringValue": json_str
            }
        }
    })
}

fn firestore_doc_to_value(doc: &Value) -> Option<Value> {
    let json_str = doc
        .get("fields")?
        .get("data")?
        .get("stringValue")?
        .as_str()?;
    serde_json::from_str(json_str).ok()
}
