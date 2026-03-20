//! Warp provider implementation
//!
//! Fetches usage data from Warp's GraphQL API
//! Requires API key for authentication

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    RateWindow, SourceMode, UsageSnapshot,
};

/// Warp GraphQL API endpoint
const WARP_API_URL: &str = "https://app.warp.dev/graphql/v2?op=GetRequestLimitInfo";

/// Windows Credential Manager target for Warp API token
const WARP_CREDENTIAL_TARGET: &str = "codexbar-warp";

/// GraphQL query for fetching request limit info
const GRAPHQL_QUERY: &str = r#"query GetRequestLimitInfo($requestContext: RequestContext!) {
  user(requestContext: $requestContext) {
    __typename
    ... on UserOutput {
      user {
        requestLimitInfo {
          isUnlimited
          nextRefreshTime
          requestLimit
          requestsUsedSinceLastRefresh
        }
        bonusGrants {
          requestCreditsGranted
          requestCreditsRemaining
          expiration
        }
        workspaces {
          bonusGrantsInfo {
            grants {
              requestCreditsGranted
              requestCreditsRemaining
              expiration
            }
          }
        }
      }
    }
  }
}"#;

/// Warp GraphQL response structures
#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    user: Option<UserWrapper>,
}

#[derive(Debug, Deserialize)]
struct UserWrapper {
    #[serde(rename = "__typename")]
    type_name: Option<String>,
    user: Option<UserData>,
}

#[derive(Debug, Deserialize)]
struct UserData {
    #[serde(rename = "requestLimitInfo")]
    request_limit_info: Option<RequestLimitInfo>,
    #[serde(rename = "bonusGrants")]
    bonus_grants: Option<Vec<BonusGrant>>,
    workspaces: Option<Vec<Workspace>>,
}

#[derive(Debug, Deserialize)]
struct RequestLimitInfo {
    #[serde(rename = "isUnlimited")]
    is_unlimited: Option<bool>,
    #[serde(rename = "nextRefreshTime")]
    next_refresh_time: Option<String>,
    #[serde(rename = "requestLimit")]
    request_limit: Option<i64>,
    #[serde(rename = "requestsUsedSinceLastRefresh")]
    requests_used: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BonusGrant {
    #[serde(rename = "requestCreditsGranted")]
    request_credits_granted: Option<i64>,
    #[serde(rename = "requestCreditsRemaining")]
    request_credits_remaining: Option<i64>,
    expiration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    #[serde(rename = "bonusGrantsInfo")]
    bonus_grants_info: Option<BonusGrantsInfo>,
}

#[derive(Debug, Deserialize)]
struct BonusGrantsInfo {
    grants: Option<Vec<BonusGrant>>,
}

/// Warp provider
pub struct WarpProvider {
    metadata: ProviderMetadata,
}

impl WarpProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Warp,
                display_name: "Warp",
                session_label: "Credits",
                weekly_label: "Add-on credits",
                supports_opus: false,
                supports_credits: false,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://docs.warp.dev/reference/cli/api-keys"),
                status_page_url: None,
            },
        }
    }

    /// Get API token from ctx, Windows Credential Manager, or env
    fn get_api_token(api_key: Option<&str>) -> Result<String, ProviderError> {
        if let Some(key) = api_key {
            if !key.is_empty() {
                return Ok(key.to_string());
            }
        }

        match keyring::Entry::new(WARP_CREDENTIAL_TARGET, "api_token") {
            Ok(entry) => match entry.get_password() {
                Ok(token) => Ok(token),
                Err(_) => std::env::var("WARP_API_KEY")
                    .or_else(|_| std::env::var("WARP_TOKEN"))
                    .map_err(|_| {
                    ProviderError::NotInstalled(
                        "Warp API key not found. Set in Preferences → Providers, WARP_API_KEY, or WARP_TOKEN environment variable.".to_string(),
                    )
                }),
            },
            Err(_) => std::env::var("WARP_API_KEY")
                .or_else(|_| std::env::var("WARP_TOKEN"))
                .map_err(|_| {
                ProviderError::NotInstalled(
                    "Warp API key not found. Set in Preferences → Providers, WARP_API_KEY, or WARP_TOKEN environment variable.".to_string(),
                )
            }),
        }
    }

    /// Fetch usage from Warp GraphQL API
    async fn fetch_usage_api(&self, ctx: &FetchContext) -> Result<UsageSnapshot, ProviderError> {
        let api_key = Self::get_api_token(ctx.api_key.as_deref())?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let os_version = "10.0";
        let body = json!({
            "query": GRAPHQL_QUERY,
            "variables": {
                "requestContext": {
                    "clientContext": {},
                    "osContext": {
                        "category": "Windows",
                        "name": "Windows",
                        "version": os_version
                    }
                }
            },
            "operationName": "GetRequestLimitInfo"
        });

        let resp = client
            .post(WARP_API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("x-warp-client-id", "warp-app")
            .header("x-warp-os-category", "Windows")
            .header("x-warp-os-name", "Windows")
            .header("x-warp-os-version", os_version)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("User-Agent", "Warp/1.0")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!(
                "Warp API returned status {}",
                resp.status()
            )));
        }

        let gql_response: GraphQLResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        // Check for GraphQL errors
        if let Some(errors) = &gql_response.errors {
            if !errors.is_empty() {
                let messages: Vec<String> =
                    errors.iter().filter_map(|e| e.message.clone()).collect();
                let summary = if messages.is_empty() {
                    "GraphQL request failed".to_string()
                } else {
                    messages.join(" | ")
                };
                return Err(ProviderError::Other(summary));
            }
        }

        let data = gql_response
            .data
            .ok_or_else(|| ProviderError::Parse("Missing data in response".to_string()))?;
        let user_wrapper = data
            .user
            .ok_or_else(|| ProviderError::Parse("Missing data.user in response".to_string()))?;
        let user = user_wrapper
            .user
            .ok_or_else(|| ProviderError::Parse("Missing user data in response".to_string()))?;
        let limit_info = user.request_limit_info.ok_or_else(|| {
            ProviderError::Parse("Missing requestLimitInfo in response".to_string())
        })?;

        let is_unlimited = limit_info.is_unlimited.unwrap_or(false);
        let request_limit = limit_info.request_limit.unwrap_or(0);
        let requests_used = limit_info.requests_used.unwrap_or(0);

        // Calculate primary usage percentage
        let used_percent = if is_unlimited {
            0.0
        } else if request_limit > 0 {
            ((requests_used as f64) / (request_limit as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        let reset_description = if is_unlimited {
            Some("Unlimited".to_string())
        } else {
            Some(format!("{}/{} credits", requests_used, request_limit))
        };

        let mut primary = RateWindow::new(used_percent);
        if let Some(desc) = reset_description {
            primary.reset_description = Some(desc);
        }

        let mut usage = UsageSnapshot::new(primary).with_login_method("Warp API");

        // Parse bonus credits (user-level + workspace-level)
        let mut all_grants: Vec<&BonusGrant> = Vec::new();
        if let Some(ref grants) = user.bonus_grants {
            all_grants.extend(grants.iter());
        }
        if let Some(ref workspaces) = user.workspaces {
            for ws in workspaces {
                if let Some(ref info) = ws.bonus_grants_info {
                    if let Some(ref grants) = info.grants {
                        all_grants.extend(grants.iter());
                    }
                }
            }
        }

        let bonus_remaining: i64 = all_grants
            .iter()
            .map(|g| g.request_credits_remaining.unwrap_or(0))
            .sum();
        let bonus_total: i64 = all_grants
            .iter()
            .map(|g| g.request_credits_granted.unwrap_or(0))
            .sum();

        if bonus_total > 0 || bonus_remaining > 0 {
            let bonus_used = bonus_total - bonus_remaining;
            let bonus_percent = if bonus_total > 0 {
                ((bonus_used as f64) / (bonus_total as f64) * 100.0).clamp(0.0, 100.0)
            } else if bonus_remaining > 0 {
                0.0
            } else {
                100.0
            };
            usage = usage.with_secondary(RateWindow::new(bonus_percent));
        }

        Ok(usage)
    }
}

impl Default for WarpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for WarpProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Warp
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Warp usage");

        match ctx.source_mode {
            SourceMode::Auto | SourceMode::OAuth => {
                let usage = self.fetch_usage_api(ctx).await?;
                Ok(ProviderFetchResult::new(usage, "api"))
            }
            SourceMode::Web | SourceMode::Cli => {
                Err(ProviderError::UnsupportedSource(ctx.source_mode))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::OAuth]
    }

    fn supports_web(&self) -> bool {
        false
    }

    fn supports_cli(&self) -> bool {
        false
    }
}
