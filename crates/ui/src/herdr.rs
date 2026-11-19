//! Herdr integration for tuirowser.
//!
//! When run inside a Herdr pane, tuirowser can:
//! - Read context about the workspace and focused agent from environment variables
//! - Know its own pane ID for self-reference
//! - Optionally deliver output back to another pane via the `herdr` CLI
//!
//! Environment variables (set by herdr when launching a plugin):
//! - `HERDR_ENV=1` — indicates we're running inside herdr
//! - `HERDR_BIN_PATH` — path to the `herdr` CLI binary
//! - `HERDR_PANE_ID` — our own pane ID
//! - `HERDR_PLUGIN_CONTEXT_JSON` — JSON context from the launching action
//! - `PLANNOTATOR_TUI_FILE` — file to open (reused convention, we use TUIROUSER_FILE)
//! - `TUIROUSER_FILE` — markdown file to open in viewer mode
//! - `TUIROUSER_URL` — URL to open in browser mode

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Context information provided by herdr when launching the plugin.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HerdrContext {
    /// The workspace ID this pane belongs to.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// The working directory of the workspace.
    #[serde(default)]
    pub workspace_cwd: Option<String>,
    /// The currently focused pane ID (may be different from ours).
    #[serde(default)]
    pub focused_pane_id: Option<String>,
    /// The agent running in the focused pane (e.g., "claude", "codex").
    #[serde(default)]
    pub focused_pane_agent: Option<String>,
    /// A URL that was clicked to open us (if applicable).
    #[serde(default)]
    pub clicked_url: Option<String>,
}

/// Environment state for herdr integration.
#[derive(Debug, Clone, Default)]
pub struct HerdrEnv {
    /// Whether we're running inside herdr.
    pub in_herdr: bool,
    /// Path to the herdr CLI binary.
    pub bin: Option<PathBuf>,
    /// Our own pane ID.
    pub pane_id: Option<String>,
    /// Context from the launching action.
    pub context: Option<HerdrContext>,
    /// File to open (from TUIROUSER_FILE env var).
    pub file: Option<PathBuf>,
    /// URL to open (from TUIROUSER_URL env var).
    pub url: Option<String>,
}

impl HerdrEnv {
    /// Detect herdr environment from process environment variables.
    pub fn detect() -> Self {
        let in_herdr = std::env::var("HERDR_ENV")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let bin = std::env::var("HERDR_BIN_PATH").ok().map(PathBuf::from);
        let pane_id = std::env::var("HERDR_PANE_ID").ok();

        let context = std::env::var("HERDR_PLUGIN_CONTEXT_JSON")
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok());

        let file = std::env::var("TUIROUSER_FILE")
            .or_else(|_| std::env::var("PLANNOTATOR_TUI_FILE"))
            .ok()
            .map(PathBuf::from);

        let url = std::env::var("TUIROUSER_URL").ok();

        Self {
            in_herdr,
            bin,
            pane_id,
            context,
            file,
            url,
        }
    }

    /// Get the workspace CWD from herdr context, if available.
    pub fn workspace_cwd(&self) -> Option<&str> {
        self.context
            .as_ref()
            .and_then(|c| c.workspace_cwd.as_deref())
    }

    /// Get the focused agent name from herdr context.
    pub fn focused_agent(&self) -> Option<&str> {
        self.context
            .as_ref()
            .and_then(|c| c.focused_pane_agent.as_deref())
    }

    /// Send a message to another herdr pane via the herdr CLI.
    ///
    /// This uses `herdr plugin pane send-text` to deliver text input to the target pane.
    pub fn send_to_pane(&self, target_pane_id: &str, text: &str) -> Result<()> {
        let bin = self
            .bin
            .as_ref()
            .context("HERDR_BIN_PATH not set; cannot send to pane")?;

        use std::process::Command;
        let output = Command::new(bin)
            .args([
                "plugin",
                "pane",
                "send-text",
                "--pane-id",
                target_pane_id,
                "--text",
                text,
            ])
            .output()
            .context("Failed to execute herdr CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("herdr pane send-text failed: {}", stderr);
        }

        Ok(())
    }

    /// Open a new herdr pane with the given command.
    pub fn open_pane(&self, command: &str) -> Result<()> {
        let bin = self
            .bin
            .as_ref()
            .context("HERDR_BIN_PATH not set; cannot open pane")?;

        use std::process::Command;
        let output = Command::new(bin)
            .args(["plugin", "pane", "open", "--command", command])
            .output()
            .context("Failed to execute herdr CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("herdr pane open failed: {}", stderr);
        }

        Ok(())
    }
}

/// Determine what to open based on herdr environment and CLI arguments.
pub struct LaunchTarget {
    /// A local file path to open.
    pub file: Option<PathBuf>,
    /// A URL to navigate to.
    pub url: Option<String>,
    /// Whether we're in herdr (affects UI chrome).
    pub in_herdr: bool,
}

impl LaunchTarget {
    /// Resolve the launch target from herdr env and CLI args.
    ///
    /// Priority: explicit file arg > TUIROUSER_FILE env > URL arg > TUIROUSER_URL env > about:blank
    pub fn resolve(cli_file: Option<&str>, cli_url: Option<&str>) -> Self {
        let herdr = HerdrEnv::detect();

        // File takes priority if explicitly given
        let file = cli_file
            .map(PathBuf::from)
            .or(herdr.file.clone());

        // URL from CLI, env, or herdr context (clicked_url)
        let url = cli_url.map(String::from)
            .or(herdr.url.clone())
            .or_else(|| herdr.context.and_then(|c| c.clicked_url));

        Self {
            file,
            url,
            in_herdr: herdr.in_herdr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_without_herdr() {
        // In test environment, HERDR_ENV is not set
        let env = HerdrEnv::detect();
        assert!(!env.in_herdr);
    }

    #[test]
    fn herdr_context_deserialization() {
        let json = r#"{"workspace_id":"ws1","workspace_cwd":"/home/user/project","focused_pane_agent":"claude"}"#;
        let ctx: HerdrContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.workspace_id.as_deref(), Some("ws1"));
        assert_eq!(ctx.focused_pane_agent.as_deref(), Some("claude"));
    }

    #[test]
    fn launch_target_priority() {
        // Without herdr env, CLI args should be used
        let target = LaunchTarget::resolve(Some("/tmp/test.md"), None);
        assert_eq!(target.file, Some(PathBuf::from("/tmp/test.md")));
        assert!(!target.in_herdr);

        let target = LaunchTarget::resolve(None, Some("https://example.com"));
        assert_eq!(target.url.as_deref(), Some("https://example.com"));
    }
}
