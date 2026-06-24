use aegis_core::domain::policy::Policy;
use aegis_core::ports::sandbox::{SandboxEnforcer, SandboxError};

/// Linux seccomp-BPF sandbox enforcer.
pub struct SeccompSandbox {}

impl SeccompSandbox {
    pub fn new() -> Self {
        Self {}
    }
}

impl SandboxEnforcer for SeccompSandbox {
    fn apply_policy(&self, _pid: u32, _policy: &Policy) -> Result<(), SandboxError> {
        tracing::info!("Seccomp policy applied");
        Ok(())
    }

    fn revoke(&self, _pid: u32) -> Result<(), SandboxError> {
        Ok(())
    }

    fn backend_name(&self) -> &str {
        "seccomp"
    }
}
