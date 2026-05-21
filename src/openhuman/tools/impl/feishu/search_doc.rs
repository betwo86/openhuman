use crate::openhuman::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::client::FeishuClient;

pub struct FeishuSearchDocTool {
    client: Option<Arc<FeishuClient>>,
}

impl FeishuSearchDocTool {
    pub fn new(client: Option<Arc<FeishuClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for FeishuSearchDocTool {
    fn name(&self) -> &str {
        "feishu_search_doc"
    }

    fn description(&self) -> &str {
        "Search for Feishu/Lark documents by title or keyword, or read document content by document token."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "read"],
                    "description": "search: find documents by keyword. read: get document content by token"
                },
                "query": {
                    "type": "string",
                    "description": "Search keyword (required for search action)"
                },
                "document_token": {
                    "type": "string",
                    "description": "Document token to read (required for read action)"
                },
                "page_size": {
                    "type": "integer",
                    "description": "Max results for search (default 10)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Feishu client not configured"))?;

        let action = args["action"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing action"))?;

        match action {
            "search" => self.search_docs(client, &args).await,
            "read" => self.read_doc(client, &args).await,
            _ => Err(anyhow::anyhow!("unsupported action: {}", action)),
        }
    }
}

impl FeishuSearchDocTool {
    async fn search_docs(&self, client: &FeishuClient, args: &Value) -> anyhow::Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing query for search action"))?;
        let page_size = args["page_size"].as_i64().unwrap_or(10).max(1).min(50);

        let encoded_query = urlencoding::encode(query);
        let resp = client
            .get(&format!(
                "/wiki/v2/search?query={}&page_size={}",
                encoded_query,
                page_size
            ))
            .await?;

        let body: Value = resp.json().await?;

        if body["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!(
                "search failed: code={}, msg={}",
                body["code"],
                body["msg"].as_str().unwrap_or("unknown")
            ));
        }

        let items = body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if items.is_empty() {
            return Ok(ToolResult::text(format!(
                "No documents found for: {}",
                query
            )));
        }

        let mut results = Vec::new();
        for item in &items {
            let title = item["title"].as_str().unwrap_or("untitled");
            let token = item["obj_token"].as_str().unwrap_or("");
            let url = item["url"].as_str().unwrap_or("");
            results.push(format!("- [{}]({}) token: {}", title, url, token));
        }

        Ok(ToolResult::text(format!(
            "Found {} documents for '{}':\n{}",
            items.len(),
            query,
            results.join("\n")
        )))
    }

    async fn read_doc(&self, client: &FeishuClient, args: &Value) -> anyhow::Result<ToolResult> {
        let token = args["document_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing document_token for read action"))?;

        let resp = client
            .get(&format!("/docx/v1/documents/{}/raw_content", token))
            .await?;

        let body: Value = resp.json().await?;

        if body["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!(
                "read doc failed: code={}, msg={}",
                body["code"],
                body["msg"].as_str().unwrap_or("unknown")
            ));
        }

        let content = body["data"]["content"]
            .as_str()
            .unwrap_or("(empty document)");

        Ok(ToolResult::text(format!("Document content:\n\n{}", content)))
    }
}

