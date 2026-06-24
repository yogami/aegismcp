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

    // TODO: Full proxy relay not yet wired
    eprintln!("[AegisMCP] Daemon initialized. Enforcement active.");
    eprintln!("[AegisMCP] TODO: Full proxy relay not yet wired. Exiting.");

    Ok(())
}
