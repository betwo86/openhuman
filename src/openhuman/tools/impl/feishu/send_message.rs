use crate::openhuman::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::client::FeishuClient;

pub struct FeishuSendMessageTool {
    client: Option<Arc<FeishuClient>>,
}

impl FeishuSendMessageTool {
    pub fn new(client: Option<Arc<FeishuClient>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for FeishuSendMessageTool {
    fn name(&self) -> &str {
        "feishu_send_message"
    }

    fn description(&self) -> &str {
        "Send a message to a Feishu/Lark chat. Supports text, post (rich text), and interactive card messages."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "receive_id": {
                    "type": "string",
                    "description": "chat ID, user ID, or open ID"
                },
                "receive_id_type": {
                    "type": "string",
                    "enum": ["chat_id", "user_id", "open_id"],
                    "description": "Type of receive_id"
                },
                "msg_type": {
                    "type": "string",
                    "enum": ["text", "post", "interactive"],
                    "description": "Message type"
                },
                "content": {
                    "type": "string",
                    "description": "Message content. For text: plain text. For post: JSON string. For interactive: card JSON string."
                }
            },
            "required": ["receive_id", "msg_type", "content"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Feishu client not configured"))?;

        let receive_id = args["receive_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing receive_id"))?;
        let receive_id_type = args["receive_id_type"]
            .as_str()
            .unwrap_or("chat_id");
        let msg_type = args["msg_type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing msg_type"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing content"))?;

        let body = json!({
            "receive_id": receive_id,
            "msg_type": msg_type,
            "content": content,
        });

        let resp = client
            .post(
                &format!("/im/v1/messages?receive_id_type={}", receive_id_type),
                &body,
            )
            .await?;

        let status = resp.status();
        let resp_body: Value = resp.json().await?;

        if status.is_success() && resp_body["code"].as_i64() == Some(0) {
            let message_id = resp_body["data"]["message_id"]
                .as_str()
                .unwrap_or("unknown");
            Ok(ToolResult::success(format!(
                "Message sent successfully. message_id: {}",
                message_id
            )))
        } else {
            Err(anyhow::anyhow!(
                "Failed to send message: code={}, msg={}",
                resp_body["code"],
                resp_body["msg"].as_str().unwrap_or("unknown")
            ))
        }
    }
}
