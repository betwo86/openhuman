use crate::openhuman::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::client::FeishuClient;

pub struct FeishuCalendarTool {
    client: Option<Arc<FeishuClient>>,
}

impl FeishuCalendarTool {
    pub fn new(client: Option<Arc<FeishuClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for FeishuCalendarTool {
    fn name(&self) -> &str {
        "feishu_calendar"
    }

    fn description(&self) -> &str {
        "Query Feishu/Lark calendar events, list calendars, and create events."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list_calendars", "list_events", "create_event"],
                    "description": "Action to perform"
                },
                "calendar_id": {
                    "type": "string",
                    "description": "Calendar ID (required for list_events and create_event)"
                },
                "start_time": {
                    "type": "string",
                    "description": "Start time in Unix timestamp seconds (for list_events)"
                },
                "end_time": {
                    "type": "string",
                    "description": "End time in Unix timestamp seconds (for list_events)"
                },
                "summary": {
                    "type": "string",
                    "description": "Event title (for create_event)"
                },
                "description": {
                    "type": "string",
                    "description": "Event description (for create_event)"
                },
                "start_time_human": {
                    "type": "string",
                    "description": "Event start time in ISO 8601 format, e.g. 2026-05-21T10:00:00 (for create_event)"
                },
                "end_time_human": {
                    "type": "string",
                    "description": "Event end time in ISO 8601 format (for create_event)"
                },
                "page_size": {
                    "type": "integer",
                    "description": "Max results (default 20)"
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
            "list_calendars" => self.list_calendars(client).await,
            "list_events" => self.list_events(client, &args).await,
            "create_event" => self.create_event(client, &args).await,
            _ => Err(anyhow::anyhow!("unsupported action: {}", action)),
        }
    }
}

impl FeishuCalendarTool {
    async fn list_calendars(&self, client: &FeishuClient) -> anyhow::Result<ToolResult> {
        let resp = client.get("/calendar/v4/calendars?page_size=50").await?;
        let body: Value = resp.json().await?;

        if body["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!(
                "list calendars failed: code={}, msg={}",
                body["code"],
                body["msg"].as_str().unwrap_or("unknown")
            ));
        }

        let items = body["data"]["calendar_list"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if items.is_empty() {
            return Ok(ToolResult::success("No calendars found."));
        }

        let mut results = Vec::new();
        for cal in &items {
            let title = cal["title"].as_str().unwrap_or("untitled");
            let id = cal["calendar_id"].as_str().unwrap_or("");
            let role = cal["role"].as_str().unwrap_or("");
            results.push(format!("- {} (id: {}, role: {})", title, id, role));
        }

        Ok(ToolResult::success(format!(
            "Found {} calendars:\n{}",
            items.len(),
            results.join("\n")
        )))
    }

    async fn list_events(&self, client: &FeishuClient, args: &Value) -> anyhow::Result<ToolResult> {
        let calendar_id = args["calendar_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing calendar_id"))?;
        let page_size = args["page_size"].as_i64().unwrap_or(20).max(1).min(100);

        let mut path = format!(
            "/calendar/v4/calendars/{}/events?page_size={}",
            calendar_id, page_size
        );

        if let Some(st) = args["start_time"].as_str() {
            path.push_str(&format!("&anchor_time={}", st));
        }

        let resp = client.get(&path).await?;
        let body: Value = resp.json().await?;

        if body["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!(
                "list events failed: code={}, msg={}",
                body["code"],
                body["msg"].as_str().unwrap_or("unknown")
            ));
        }

        let items = body["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if items.is_empty() {
            return Ok(ToolResult::success("No events found."));
        }

        let mut results = Vec::new();
        for ev in &items {
            let summary = ev["summary"].as_str().unwrap_or("(no title)");
            let start = ev["start_time"]["timestamp"]
                .as_str()
                .unwrap_or("unknown");
            let end = ev["end_time"]["timestamp"].as_str().unwrap_or("");
            results.push(format!(
                "- {} ({} -> {})",
                summary, start, end
            ));
        }

        Ok(ToolResult::success(format!(
            "Found {} events:\n{}",
            items.len(),
            results.join("\n")
        )))
    }

    async fn create_event(&self, client: &FeishuClient, args: &Value) -> anyhow::Result<ToolResult> {
        let calendar_id = args["calendar_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing calendar_id"))?;
        let summary = args["summary"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing summary"))?;
        let start_human = args["start_time_human"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing start_time_human"))?;
        let end_human = args["end_time_human"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing end_time_human"))?;
        let description = args["description"].as_str().unwrap_or("");

        let body = json!({
            "summary": summary,
            "description": description,
            "start_time": {
                "timestamp": start_human,
                "timezone": "Asia/Shanghai"
            },
            "end_time": {
                "timestamp": end_human,
                "timezone": "Asia/Shanghai"
            }
        });

        let resp = client
            .post(&format!("/calendar/v4/calendars/{}/events", calendar_id), &body)
            .await?;

        let resp_body: Value = resp.json().await?;

        if resp_body["code"].as_i64() == Some(0) {
            let event_id = resp_body["data"]["event_id"]
                .as_str()
                .unwrap_or("unknown");
            Ok(ToolResult::success(format!(
                "Event created successfully. event_id: {}",
                event_id
            )))
        } else {
            Err(anyhow::anyhow!(
                "create event failed: code={}, msg={}",
                resp_body["code"],
                resp_body["msg"].as_str().unwrap_or("unknown")
            ))
        }
    }
}
