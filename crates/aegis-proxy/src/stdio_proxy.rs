use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("Failed to spawn process: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Missing stdio handles")]
    MissingStdio,
}

pub struct StdioProxy {
    config: ProxyConfig,
    child: Option<Child>,
}

impl StdioProxy {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            child: None,
        }
    }

    pub fn spawn(&mut self) -> Result<(), ProxyError> {
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .envs(&self.config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.child = Some(child);
        Ok(())
    }

    pub async fn relay_with_intercept<F>(&mut self, mut on_message: F) -> Result<(), ProxyError>
    where
        F: FnMut(&serde_json::Value) + Send + 'static,
    {
        let child = self.child.as_mut().ok_or(ProxyError::MissingStdio)?;
        let mut stdin = child.stdin.take().ok_or(ProxyError::MissingStdio)?;
        let stdout = child.stdout.take().ok_or(ProxyError::MissingStdio)?;
        let mut stderr = child.stderr.take().ok_or(ProxyError::MissingStdio)?;

        // Relay parent stdin -> child stdin
        let mut parent_stdin = tokio::io::stdin();
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut parent_stdin, &mut stdin).await;
        });

        // Relay child stderr -> parent stderr
        let mut parent_stderr = tokio::io::stderr();
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut stderr, &mut parent_stderr).await;
        });

        // Relay child stdout -> parent stdout + intercept
        let mut reader = BufReader::new(stdout);
        let mut parent_stdout = tokio::io::stdout();
        let mut line = String::new();

        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                on_message(&msg);
            }
            let _ = parent_stdout.write_all(line.as_bytes()).await;
            let _ = parent_stdout.flush().await;
            line.clear();
        }

        let _ = child.wait().await;
        Ok(())
    }
}
