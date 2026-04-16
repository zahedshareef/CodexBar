use crate::commands::ProviderCatalogEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayMenuEntry {
    pub(crate) id: Option<String>,
    pub(crate) label: String,
    pub(crate) children: Vec<Self>,
    pub(crate) is_separator: bool,
}

impl TrayMenuEntry {
    fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            children: Vec::new(),
            is_separator: false,
        }
    }

    fn submenu(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            id: None,
            label: label.into(),
            children,
            is_separator: false,
        }
    }

    fn separator() -> Self {
        Self {
            id: None,
            label: String::new(),
            children: Vec::new(),
            is_separator: true,
        }
    }
}

pub(crate) fn build_tray_menu(providers: &[ProviderCatalogEntry]) -> Vec<TrayMenuEntry> {
    let mut menu = vec![
        TrayMenuEntry::item("show_panel", "Show Panel"),
        TrayMenuEntry::item("pop_out", "Pop Out Dashboard"),
        TrayMenuEntry::item("settings", "Settings"),
        TrayMenuEntry::item("about", "About"),
        TrayMenuEntry::separator(),
    ];
    if !providers.is_empty() {
        menu.push(TrayMenuEntry::submenu(
            "Providers",
            providers
                .iter()
                .map(|provider| {
                    TrayMenuEntry::item(
                        format!("provider:{}", provider.id),
                        format!("Open {}", provider.display_name),
                    )
                })
                .collect(),
        ));
        menu.push(TrayMenuEntry::separator());
    }
    menu.extend([
        TrayMenuEntry::item("refresh", "Refresh All"),
        TrayMenuEntry::separator(),
        TrayMenuEntry::item("quit", "Quit CodexBar"),
    ]);
    menu
}

pub(crate) fn proof_menu_items(entries: &[TrayMenuEntry]) -> Vec<String> {
    let mut items = Vec::new();
    push_proof_menu_items(entries, None, &mut items);
    items
}

fn push_proof_menu_items(entries: &[TrayMenuEntry], prefix: Option<&str>, items: &mut Vec<String>) {
    for entry in entries {
        if entry.is_separator {
            continue;
        }

        if entry.children.is_empty() {
            let label = match prefix {
                Some(prefix) => format!("{prefix}/{}", entry.label),
                None => entry.label.clone(),
            };
            items.push(label);
            continue;
        }

        let next_prefix = match prefix {
            Some(prefix) => format!("{prefix}/{}", entry.label),
            None => entry.label.clone(),
        };
        push_proof_menu_items(&entry.children, Some(next_prefix.as_str()), items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provider_catalog() -> Vec<ProviderCatalogEntry> {
        vec![
            ProviderCatalogEntry {
                id: "codex".into(),
                display_name: "Codex".into(),
                cookie_domain: None,
            },
            ProviderCatalogEntry {
                id: "claude".into(),
                display_name: "Claude".into(),
                cookie_domain: None,
            },
        ]
    }

    #[test]
    fn proof_menu_items_follow_shared_spec() {
        let items = proof_menu_items(&build_tray_menu(&sample_provider_catalog()));

        assert_eq!(
            items,
            vec![
                "Show Panel",
                "Pop Out Dashboard",
                "Settings",
                "About",
                "Providers/Open Codex",
                "Providers/Open Claude",
                "Refresh All",
                "Quit CodexBar",
            ]
        );
    }
}
