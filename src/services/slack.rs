use super::{CallError, FactoryError, Notification, Service, ServiceFactory};
use crate::k8s::secrets::get_secret;
use async_trait::async_trait;
use parking_lot::Mutex;
use reqwest::header;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Deserialize)]
struct ServiceConfig {
    token: String,
    icon_emoji: Option<String>,
}

impl ServiceConfig {
    fn from_value(value: serde_json::Value) -> Result<Self, FactoryError> {
        serde_json::from_value(value).map_err(|e| FactoryError::ConfigError(e.to_string()))
    }
}

#[derive(Deserialize)]
struct NotificationConfig {
    channel: String,
}

impl NotificationConfig {
    fn from_value(value: serde_json::Value) -> Result<Self, CallError> {
        serde_json::from_value(value).map_err(|e| CallError::ConfigError(e.to_string()))
    }
}

#[derive(Clone)]
struct Channel {
    channel_id: String,
    thread_id: String,
}

#[derive(Deserialize)]
struct TokenSecret {
    token: String,
}

pub struct SlackFactory;

#[async_trait]
impl ServiceFactory for SlackFactory {
    async fn from_config(config: serde_json::Value) -> Result<Arc<dyn Service>, FactoryError> {
        let config = ServiceConfig::from_value(config)?;
        let token_secret: TokenSecret = get_secret(&config.token)
            .await
            .map_err(|e| FactoryError::ConfigError(format!("Invalid token secret: {}", e)))?;
        Ok(Arc::new(Slack {
            config: ServiceConfig {
                icon_emoji: config.icon_emoji,
                token: token_secret.token,
            },
            channels: Arc::new(Mutex::new(HashMap::new())),
        }))
    }
}

#[derive(Debug, Deserialize)]
struct SlackSuccessResponse {
    ok: bool,
    channel: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct SlackErrorResponse {
    ok: bool,
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SlackResponse {
    Success(SlackSuccessResponse),
    Error(SlackErrorResponse),
}

#[derive(Debug, Deserialize)]
struct RenderedTemplate {
    text: Option<String>,
    blocks: Option<serde_json::Value>,
}

pub struct Slack {
    config: ServiceConfig,
    channels: Arc<Mutex<HashMap<String, Box<Channel>>>>,
}

impl Slack {
    fn get_channel(&self, name: &str) -> Option<Box<Channel>> {
        self.channels.lock().get(name).cloned()
    }

    fn update_channel(&self, name: &str, channel: Channel) {
        let mut channels = self.channels.lock();
        channels.insert(name.into(), Box::from(channel));
    }

    fn render(
        &self,
        notification: &Notification,
        subtemplate: &str,
    ) -> Result<RenderedTemplate, CallError> {
        let raw_template = notification
            .render(subtemplate)
            .map_err(|e| CallError::RenderError(e.to_string()))?;
        serde_json::from_str(raw_template.as_str())
            .map_err(|e| CallError::RenderError(e.to_string()))
    }

    async fn post(
        &self,
        call: &str,
        payload: &serde_json::Value,
    ) -> Result<SlackSuccessResponse, String> {
        let url = format!("https://slack.com/api/{}", call);
        let response: SlackResponse = reqwest::Client::new()
            .post(url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.config.token),
            )
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Unexpected error: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Slack response parsing error: {}", e))?;

        match response {
            SlackResponse::Success(r) if r.ok => Ok(r),
            SlackResponse::Error(r) if !r.ok => Err(format!("{} {}", r.error, self.config.token)),
            _ => unreachable!(),
        }
    }
}

#[async_trait]
impl Service for Slack {
    async fn notify(
        &self,
        config: serde_json::Value,
        notification: Notification,
    ) -> Result<(), CallError> {
        // TODO: There is a potential fast-path here that could be taken for the case when the
        // channel ID and thread ID are known (for subsequent notifications in the same channel).
        // As they are known, we could issue the primary and secondary notifications in parallel.
        let notification_config = NotificationConfig::from_value(config)?;

        // Retrieve the cached data about the channel, if any
        let channel_data = self.get_channel(&notification_config.channel);
        let channel = channel_data
            .as_ref()
            .map(|c| c.channel_id.clone())
            .unwrap_or_else(|| notification_config.channel.clone());
        let thread_id = channel_data.as_ref().map(|c| c.thread_id.clone());

        // Create new or update the existing primary notification
        let template = self.render(&notification, "primary")?;
        let payload = serde_json::json!({
            "channel": channel,
            "ts": thread_id,
            "icon_emoji": self.config.icon_emoji,
            "text": template.text,
            "blocks": template.blocks,
        });
        let call = thread_id
            .and(Some("chat.update"))
            .unwrap_or("chat.postMessage");
        let SlackSuccessResponse {
            ts: thread_id,
            channel,
            ..
        } = self.post(call, &payload).await.map_err(CallError::Fail)?;

        // Create new secondary notification (a thread message)
        let template = self.render(&notification, "secondary")?;
        let payload = serde_json::json!({
            "channel": channel,
            "thread_ts": thread_id,
            "icon_emoji": self.config.icon_emoji,
            "text": template.text,
            "blocks": template.blocks,
        });
        self.post("chat.postMessage", &payload)
            .await
            .map_err(CallError::Fail)?;

        // Update the cache if needed
        if channel_data.is_none() {
            self.update_channel(
                &notification_config.channel,
                Channel {
                    channel_id: channel,
                    thread_id,
                },
            )
        };

        Ok(())
    }
}
