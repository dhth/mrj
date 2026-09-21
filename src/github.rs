use reqwest::header::{
    ACCEPT, AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue,
};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub(crate) enum GitHubClientBuildError {
    #[error("couldn't build GitHub authorization header")]
    InvalidAuthorizationHeader(#[from] InvalidHeaderValue),
    #[error("couldn't build GitHub HTTP client")]
    BuildClient(#[from] reqwest::Error),
}

#[derive(Debug)]
pub(crate) struct GitHubClient {
    client: reqwest::Client,
}

impl GitHubClient {
    pub(crate) fn new(token: &str) -> Result<Self, GitHubClientBuildError> {
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))?;
        authorization.set_sensitive(true);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            HeaderName::from_static("x-github-api-version"),
            HeaderValue::from_static("2026-03-10"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("mrj")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self { client })
    }
}

#[cfg(test)]
mod tests {
    use super::{GitHubClient, GitHubClientBuildError};

    #[test]
    fn client_cannot_be_created_with_an_invalid_token() {
        // GIVEN
        let token = "github-token\nsecret";
        let result = GitHubClient::new(token);

        // WHEN
        let error = result.expect_err("result should've been an error");

        // THEN
        assert!(matches!(
            error,
            GitHubClientBuildError::InvalidAuthorizationHeader(_)
        ));
    }
}
