//! Email delivery via Resend API.
//!
//! When `NEXUS__EMAIL__API_KEY` is set, emails are sent through the Resend
//! REST API (https://resend.com/docs/api-reference/emails/send-email).
//! When unset, email sending is a no-op and verification tokens are logged
//! at DEBUG level for development convenience.

use nexus_common::config::EmailConfig;
use serde::Serialize;

/// Lightweight email sender backed by Resend's HTTP API.
#[derive(Clone)]
pub struct EmailService {
    client: reqwest::Client,
    config: EmailConfig,
}

#[derive(Serialize)]
struct ResendPayload {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Returns `true` when a valid Resend API key is configured.
    pub fn is_enabled(&self) -> bool {
        self.config.is_enabled()
    }

    /// Send an email via Resend. Returns `Ok(())` on success, or an error string.
    /// If email is not configured, this is a no-op that returns `Ok(())`.
    async fn send(&self, to: &str, subject: &str, html: &str) -> Result<(), String> {
        if !self.is_enabled() {
            tracing::debug!(to, subject, "Email not sent (no API key configured)");
            return Ok(());
        }

        let payload = ResendPayload {
            from: self.config.from.clone(),
            to: vec![to.to_owned()],
            subject: subject.to_owned(),
            html: html.to_owned(),
        };

        let resp = self
            .client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send email: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status = %status, body, "Resend API error");
            return Err(format!("Resend API returned {status}"));
        }

        tracing::info!(to, subject, "Email sent via Resend");
        Ok(())
    }

    /// Send a verification email with a clickable link.
    ///
    /// The `raw_token` is the unhashed token stored in the database row.
    /// `base_url` is read from `EmailConfig.base_url`.
    pub async fn send_verification_email(
        &self,
        to_email: &str,
        username: &str,
        raw_token: &str,
    ) -> Result<(), String> {
        if !self.is_enabled() {
            tracing::debug!(
                to = to_email,
                token = raw_token,
                "Verification email not sent (no API key configured — token logged for dev)"
            );
            return Ok(());
        }

        let base = self.config.base_url.trim_end_matches('/');
        let verify_url = format!("{base}/api/v1/auth/verify-email?token={raw_token}");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a2e; color: #e0e0e0; padding: 40px;">
  <div style="max-width: 480px; margin: 0 auto; background: #16213e; border-radius: 12px; padding: 32px; text-align: center;">
    <div style="width: 56px; height: 56px; border-radius: 14px; background: #7c3aed; display: inline-flex; align-items: center; justify-content: center; margin-bottom: 16px;">
      <span style="color: white; font-size: 28px; font-weight: bold;">N</span>
    </div>
    <h1 style="margin: 0 0 8px; font-size: 22px; color: #fff;">Verify your email</h1>
    <p style="color: #a0a0b0; font-size: 14px; margin: 0 0 24px;">
      Hey <strong style="color:#fff;">{username}</strong>, click the button below to verify your email address and unlock full access to Nexus.
    </p>
    <a href="{verify_url}" style="display: inline-block; background: #7c3aed; color: white; text-decoration: none; padding: 12px 32px; border-radius: 8px; font-weight: 600; font-size: 15px;">
      Verify Email
    </a>
    <p style="color: #6b6b7b; font-size: 12px; margin: 24px 0 0;">
      If you didn't create a Nexus account, you can safely ignore this email.<br>
      This link expires in 24 hours.
    </p>
  </div>
</body>
</html>"#
        );

        self.send(to_email, "Verify your Nexus email", &html).await
    }

    /// Send a password reset email with a clickable one-time link.
    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        username: &str,
        raw_token: &str,
    ) -> Result<(), String> {
        if !self.is_enabled() {
            tracing::debug!(
                to = to_email,
                token = raw_token,
                "Password reset email not sent (no API key configured — token logged for dev)"
            );
            return Ok(());
        }

        let base = self.config.base_url.trim_end_matches('/');
        let reset_url = format!("{base}/reset-password?token={raw_token}");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset=\"utf-8\"></head>
<body style=\"font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a2e; color: #e0e0e0; padding: 40px;\">
    <div style=\"max-width: 480px; margin: 0 auto; background: #16213e; border-radius: 12px; padding: 32px; text-align: center;\">
        <h1 style=\"margin: 0 0 8px; font-size: 22px; color: #fff;\">Reset your password</h1>
        <p style=\"color: #a0a0b0; font-size: 14px; margin: 0 0 24px;\">
            Hey <strong style=\"color:#fff;\">{username}</strong>, click below to reset your Nexus password.
            This link expires in 2 hours and can only be used once.
        </p>
        <a href=\"{reset_url}\" style=\"display: inline-block; background: #7c3aed; color: white; text-decoration: none; padding: 12px 32px; border-radius: 8px; font-weight: 600; font-size: 15px;\">
            Reset Password
        </a>
        <p style=\"color: #6b6b7b; font-size: 12px; margin: 24px 0 0;\">
            If you didn't request this, you can safely ignore this email.
        </p>
    </div>
</body>
</html>"#
        );

        self.send(to_email, "Reset your Nexus password", &html)
            .await
    }
}
