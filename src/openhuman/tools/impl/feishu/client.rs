use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const CN_BASE_URL: &str = "https://open.feishu.cn/open-apis";
const INT_BASE_URL: &str = "https://open.larksuite.com/open-apis";
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(300);

pub struct FeishuClient {
    app_id: String,
    app_secret: String,
    base_url: &'static str,
    http: reqwest::Client,
    token_cache: RwLock<TokenCache>,
}

struct TokenCache {
    token: String,
    expires_at: Instant,
}

impl FeishuClient {
    pub fn new(app_id: String, app_secret: String, use_feishu: bool) -> Arc<Self> {
        Arc::new(Self {
            app_id,
            app_secret,
            base_url: if use_feishu { CN_BASE_URL } else { INT_BASE_URL },
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest Client::new"),
            token_cache: RwLock::new(TokenCache {
                token: String::new(),
                expires_at: Instant::now(),
            }),
        })
    }

    pub async fn get_tenant_token(&self) -> anyhow::Result<String> {
        {
            let cache = self.token_cache.read().await;
            if !cache.token.is_empty() && Instant::now() < cache.expires_at {
                return Ok(cache.token.clone());
            }
        }

        let resp = self
            .http
            .post(format!("{}/auth/v3/tenant_access_token/internal", self.base_url))
            .json(&serde_json::json!({
                "app_id": self.app_id,
                "app_secret": self.app_secret,
            }))
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        let token = body["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("feishu auth failed: {:?}", body))?
            .to_string();

        let expire = body["expire"]
            .as_i64()
            .unwrap_or(7200)
            .max(60) as u64;

        let mut cache = self.token_cache.write().await;
        cache.token = token.clone();
        cache.expires_at = Instant::now() + Duration::from_secs(expire) - TOKEN_REFRESH_MARGIN;

        Ok(token)
    }

    pub async fn get(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        let token = self.get_tenant_token().await?;
        Ok(self
            .http
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?)
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> anyhow::Result<reqwest::Response> {
        let token = self.get_tenant_token().await?;
        Ok(self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await?)
    }
}
