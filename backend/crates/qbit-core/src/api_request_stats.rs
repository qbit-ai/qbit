use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderRequestStatsSnapshot {
    pub requests: u64,
    pub last_sent_at: Option<u64>,
    pub last_received_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiRequestStatsSnapshot {
    pub providers: HashMap<String, ProviderRequestStatsSnapshot>,
}

#[derive(Debug, Default)]
struct ProviderRequestStats {
    requests: u64,
    last_sent_at: Option<u64>,
    last_received_at: Option<u64>,
}

#[derive(Debug, Default)]
pub struct ApiRequestStats {
    providers: RwLock<HashMap<String, ProviderRequestStats>>,
}

impl ApiRequestStats {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn record_sent(&self, provider: &str) {
        let now = now_ms();
        let mut providers = self.providers.write().await;
        let stats = providers.entry(provider.to_string()).or_default();
        stats.requests = stats.requests.saturating_add(1);
        stats.last_sent_at = Some(now);
    }

    pub async fn record_received(&self, provider: &str) {
        let now = now_ms();
        let mut providers = self.providers.write().await;
        let stats = providers.entry(provider.to_string()).or_default();
        stats.last_received_at = Some(now);
    }

    pub async fn snapshot(&self) -> ApiRequestStatsSnapshot {
        let providers = self.providers.read().await;
        ApiRequestStatsSnapshot {
            providers: providers
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        ProviderRequestStatsSnapshot {
                            requests: v.requests,
                            last_sent_at: v.last_sent_at,
                            last_received_at: v.last_received_at,
                        },
                    )
                })
                .collect(),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
