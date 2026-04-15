use serde::{Deserialize, Serialize};

use crate::surface::SurfaceMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurfaceTarget {
    #[default]
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

impl SurfaceTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "summary" => Some(Self::Summary),
            "dashboard" => Some(Self::Dashboard),
            _ => {
                if let Some(provider_id) = s.strip_prefix("provider:")
                    && !provider_id.is_empty()
                {
                    return Some(Self::Provider {
                        provider_id: provider_id.to_string(),
                    });
                }

                if let Some(tab) = s.strip_prefix("settings:")
                    && !tab.is_empty()
                {
                    return Some(Self::Settings {
                        tab: tab.to_string(),
                    });
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

    pub fn mode(&self) -> SurfaceMode {
        match self {
            Self::Summary => SurfaceMode::TrayPanel,
            Self::Dashboard | Self::Provider { .. } => SurfaceMode::PopOut,
            Self::Settings { .. } => SurfaceMode::Settings,
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

    #[test]
    fn target_mode_matches_surface_mode() {
        assert_eq!(
            SurfaceTarget::Summary.mode(),
            crate::surface::SurfaceMode::TrayPanel
        );
        assert_eq!(
            SurfaceTarget::Dashboard.mode(),
            crate::surface::SurfaceMode::PopOut
        );
        assert_eq!(
            SurfaceTarget::Provider {
                provider_id: "claude".into()
            }
            .mode(),
            crate::surface::SurfaceMode::PopOut
        );
        assert_eq!(
            SurfaceTarget::Settings {
                tab: "about".into()
            }
            .mode(),
            crate::surface::SurfaceMode::Settings
        );
    }
}
