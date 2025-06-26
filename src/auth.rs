use anyhow::Context;
use std::env::VarError;
use std::process::Command;

const TOKEN_ENV_VAR: &str = "MRJ_TOKEN";

pub fn get_token() -> anyhow::Result<String> {
    let token = std::env::var(TOKEN_ENV_VAR).or_else(|err|
        match err {
                VarError::NotPresent => get_token_from_gh().context(
                format!(r#"couldn't get a GitHub authentication token

mrj tries to get this token in the following order:
- Using the value of environment variable {TOKEN_ENV_VAR} (this was not set)
- Running "gh auth token" (this failed)

Make sure mrj can get a token from either one of these approaches, and that the token has the following permissions for the relevant repos:
- Read access to checks, metadata, and pull requests
- Read and write access to code"#
            )),
            VarError::NotUnicode(_) => Err(anyhow::anyhow!("{} is not valid unicode", TOKEN_ENV_VAR))
        })?;

    Ok(token)
}

fn get_token_from_gh() -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("couldn't get token from \"gh\"")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "couldn't get token from \"gh\"; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}
