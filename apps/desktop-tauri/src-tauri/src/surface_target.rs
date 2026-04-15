use serde::{Deserialize, Serialize};

use crate::surface::SurfaceMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurfaceTarget {
    Summary,
    Dashboard,
    Provider {
        #[serde(rename = "providerId")]
        provider_id: String,
    },
    Settings {
        tab: String,
    },
}

impl Default for SurfaceTarget {
    fn default() -> Self {
        Self::Summary
    }
}

impl SurfaceTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "summary" => Some(Self::Summary),
            "dashboard" => Some(Self::Dashboard),
            _ => {
                if let Some(provider_id) = s.strip_prefix("provider:") {
                    if !provider_id.is_empty() {
                        return Some(Self::Provider {
                            provider_id: provider_id.to_string(),
                        });
                    }
                }

                if let Some(tab) = s.strip_prefix("settings:") {
                    if !tab.is_empty() {
                        return Some(Self::Settings {
                            tab: tab.to_string(),
                        });
                    }
                }

                None
            }
        }
    }

    pub fn default_for_mode(mode: SurfaceMode) -> Self {
        match mode {
            SurfaceMode::Hidden | SurfaceMode::TrayPanel => Self::Summary,
            SurfaceMode::PopOut => Self::Dashboard,
            SurfaceMode::Settings => Self::Settings {
                tab: "general".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SurfaceTarget;
    use serde_json::json;

    #[test]
    fn parse_provider_target() {
        let target = SurfaceTarget::parse("provider:codex").unwrap();
        assert_eq!(
            target,
            SurfaceTarget::Provider {
                provider_id: "codex".into()
            }
        );
    }

    #[test]
    fn parse_settings_about_target() {
        let target = SurfaceTarget::parse("settings:about").unwrap();
        assert_eq!(
            target,
            SurfaceTarget::Settings {
                tab: "about".into()
            }
        );
    }

    #[test]
    fn serialize_provider_target_for_bridge() {
        let value = serde_json::to_value(SurfaceTarget::Provider {
            provider_id: "codex".into(),
        })
        .unwrap();

        assert_eq!(value, json!({ "kind": "provider", "providerId": "codex" }));
    }
}
