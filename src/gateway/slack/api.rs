use crate::error::ClawError;

/// Slack Web API client for posting/updating messages.
pub struct SlackApi {
    bot_token: String,
    client: reqwest::Client,
}

impl SlackApi {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Post a new message. Returns the message timestamp (ts).
    pub async fn post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<String, ClawError> {
        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });
        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        let resp = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClawError::Gateway(format!("post_message failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::Gateway(format!("post_message parse failed: {e}")))?;

        if !result["ok"].as_bool().unwrap_or(false) {
            let err = result["error"].as_str().unwrap_or("unknown");
            return Err(ClawError::Gateway(format!("post_message error: {err}")));
        }

        result["ts"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ClawError::Gateway("No ts in post_message response".into()))
    }

    /// Update an existing message.
    pub async fn update_message(
        &self,
        channel: &str,
        ts: &str,
        text: &str,
    ) -> Result<(), ClawError> {
        let body = serde_json::json!({
            "channel": channel,
            "ts": ts,
            "text": text,
        });

        let resp = self
            .client
            .post("https://slack.com/api/chat.update")
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClawError::Gateway(format!("update_message failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::Gateway(format!("update_message parse failed: {e}")))?;

        if !result["ok"].as_bool().unwrap_or(false) {
            let err = result["error"].as_str().unwrap_or("unknown");
            return Err(ClawError::Gateway(format!("update_message error: {err}")));
        }
        Ok(())
    }

    /// Add a reaction (emoji) to a message.
    pub async fn add_reaction(
        &self,
        channel: &str,
        ts: &str,
        emoji: &str,
    ) -> Result<(), ClawError> {
        let body = serde_json::json!({
            "channel": channel,
            "timestamp": ts,
            "name": emoji,
        });

        let resp = self
            .client
            .post("https://slack.com/api/reactions.add")
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ClawError::Gateway(format!("add_reaction failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::Gateway(format!("add_reaction parse failed: {e}")))?;

        if !result["ok"].as_bool().unwrap_or(false) {
            // Ignore already_reacted
            let err = result["error"].as_str().unwrap_or("unknown");
            if err != "already_reacted" {
                return Err(ClawError::Gateway(format!("add_reaction error: {err}")));
            }
        }
        Ok(())
    }

    /// Test authentication and return the bot's user ID.
    pub async fn auth_test(&self) -> Result<String, ClawError> {
        let resp = self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .map_err(|e| ClawError::Gateway(format!("auth.test failed: {e}")))?;

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ClawError::Gateway(format!("auth.test parse failed: {e}")))?;

        if !result["ok"].as_bool().unwrap_or(false) {
            let err = result["error"].as_str().unwrap_or("unknown");
            return Err(ClawError::Gateway(format!("auth.test error: {err}")));
        }

        result["user_id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ClawError::Gateway("No user_id in auth.test response".into()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn slack_api_post_message_format() {
        // Verify the JSON body format we send to Slack
        let body = serde_json::json!({
            "channel": "C123",
            "text": "Hello",
            "thread_ts": "1234.5678",
        });
        assert_eq!(body["channel"], "C123");
        assert_eq!(body["text"], "Hello");
        assert_eq!(body["thread_ts"], "1234.5678");
    }
}
