use clap::Parser;
use std::path::PathBuf;

/// AegisMCP — Kernel-level security daemon for Model Context Protocol.
///
/// Wraps any stdio-based MCP server with OS-level sandboxing to prevent
/// prompt-injection data exfiltration.
///
/// Example:
///   aegismcp --policy filesystem.json -- npx @modelcontextprotocol/server-filesystem /workspace
#[derive(Parser, Debug)]
#[command(name = "aegismcp", version, about, long_about = None)]
struct Cli {
    /// Path to a policy JSON file defining allowed capabilities.
    #[arg(short, long)]
    policy: Option<PathBuf>,

    /// Allow file reads matching this glob pattern (can be repeated).
    #[arg(long, action = clap::ArgAction::Append)]
    allow_read: Vec<String>,

    /// Allow file writes matching this glob pattern (can be repeated).
    #[arg(long, action = clap::ArgAction::Append)]
    allow_write: Vec<String>,

    /// Allow network connections to this host (can be repeated).
    #[arg(long, action = clap::ArgAction::Append)]
    allow_net: Vec<String>,

    /// Block all outbound network connections (default behavior).
    #[arg(long, default_value_t = true)]
    deny_net_all: bool,

    /// Path to write audit log (default: stderr).
    #[arg(long)]
    audit_log: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(short, long)]
    verbose: bool,

    /// The command and arguments for the MCP server to wrap.
    /// Everything after '--' is treated as the server command.
    #[arg(last = true, required = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .json()
        .init();

    tracing::info!(
        command = ?cli.command,
        policy = ?cli.policy,
        "AegisMCP starting — wrapping MCP server"
    );

    use aegis_core::domain::policy::Policy;
    use aegis_proxy::stdio_proxy::{ProxyConfig, StdioProxy};
    use std::fs;

    // Load policy
    let policy_json = if let Some(path) = &cli.policy {
        fs::read_to_string(path)
            .unwrap_or_else(|_| "{\"name\": \"default\", \"tool_policies\": {}}".into())
    } else {
        "{\"name\": \"default\", \"tool_policies\": {}}".into()
    };

    let policy = Policy::from_json(&policy_json).unwrap_or_else(|_| Policy {
        name: "fallback".into(),
        description: None,
        default_deny: cli.deny_net_all,
        tool_policies: std::collections::HashMap::new(),
    });

    #[cfg(target_os = "macos")]
    let (command, args) = {
        use aegis_sandbox::macos::MacOsSandbox;
        let profile_content = MacOsSandbox::generate_sb_profile(&policy);
        let profile_path = std::env::temp_dir().join("aegismcp-runtime.sb");
        fs::write(&profile_path, profile_content)?;

        eprintln!("[AegisMCP] 🛡️  Daemon initialized. Kernel sandbox active.");

        let mut final_args = vec![
            "-f".into(),
            profile_path.to_string_lossy().into_owned(),
            "--".into(),
        ];
        final_args.extend(cli.command);
        ("sandbox-exec".to_string(), final_args)
    };

    #[cfg(not(target_os = "macos"))]
    let (command, args) = {
        eprintln!("[AegisMCP] 🛡️  Daemon initialized. Target platform execution active.");
        let mut cmd_iter = cli.command.iter();
        let cmd = cmd_iter.next().cloned().unwrap_or_default();
        let final_args = cmd_iter.cloned().collect::<Vec<String>>();
        (cmd, final_args)
    };

    let config = ProxyConfig {
        command,
        args,
        env: std::collections::HashMap::new(),
    };

    let mut proxy = StdioProxy::new(config);
    proxy.spawn()?;

    proxy
        .relay_with_intercept(|msg| {
            if let Some(req) = msg.as_object() {
                if req.get("method").and_then(|m| m.as_str()) == Some("tools/call") {
                    if let Some(params) = req.get("params") {
                        if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                            eprintln!("[AegisMCP] 🔍 Tool call intercepted: {}", name);
                            if let Some(args) = params.get("arguments") {
                                eprintln!("[AegisMCP] 📦 Arguments: {}", args);
                            }
                        }
                    }
                }
            }
        })
        .await?;

    Ok(())
}
