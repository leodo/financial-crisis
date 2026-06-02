use anyhow::{bail, Result};

pub(crate) async fn handle_audit_command(action: &str, rest: &[String]) -> Result<()> {
    match action {
        "export-current" => crate::export_current_audit(rest).await,
        _ => {
            super::print_help();
            bail!("unknown audit command: {action}")
        }
    }
}
