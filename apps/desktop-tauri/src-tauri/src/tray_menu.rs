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

    fn path_segment(&self) -> Option<String> {
        if self.is_separator {
            return None;
        }

        Some(
            self.id
                .clone()
                .unwrap_or_else(|| self.label.to_ascii_lowercase().replace(' ', "_")),
        )
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

pub(crate) fn proof_menu_items(entries: &[TrayMenuEntry], menu_path: &str) -> Option<Vec<String>> {
    proof_menu_entries(entries, menu_path).map(|visible_entries| {
        visible_entries
            .iter()
            .filter(|entry| !entry.is_separator)
            .map(|entry| entry.label.clone())
            .collect()
    })
}

pub(crate) fn proof_menu_context_for_item(
    entries: &[TrayMenuEntry],
    item_id: &str,
) -> Option<(String, Vec<String>)> {
    proof_menu_context_for_item_inner(entries, item_id, "tray")
}

fn proof_menu_context_for_item_inner(
    entries: &[TrayMenuEntry],
    item_id: &str,
    menu_path: &str,
) -> Option<(String, Vec<String>)> {
    for entry in entries {
        if entry.is_separator {
            continue;
        }

        if entry.id.as_deref() == Some(item_id) {
            return proof_menu_items(entries, menu_path)
                .map(|items| (menu_path.to_string(), items));
        }

        if entry.children.is_empty() {
            continue;
        }

        let next_path = format!("{menu_path}/{}", entry.path_segment()?);
        if let Some(context) =
            proof_menu_context_for_item_inner(&entry.children, item_id, &next_path)
        {
            return Some(context);
        }
    }

    None
}

fn proof_menu_entries<'a>(
    entries: &'a [TrayMenuEntry],
    menu_path: &str,
) -> Option<&'a [TrayMenuEntry]> {
    let mut segments = menu_path.split('/');
    if segments.next()? != "tray" {
        return None;
    }

    let mut current = entries;
    for segment in segments {
        let submenu = current.iter().find(|entry| {
            !entry.is_separator
                && !entry.children.is_empty()
                && entry.path_segment().as_deref() == Some(segment)
        })?;
        current = &submenu.children;
    }

    Some(current)
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
    fn proof_menu_items_follow_current_context() {
        let items = proof_menu_items(&build_tray_menu(&sample_provider_catalog()), "tray").unwrap();

        assert_eq!(
            items,
            vec![
                "Show Panel",
                "Pop Out Dashboard",
                "Settings",
                "About",
                "Providers",
                "Refresh All",
                "Quit CodexBar",
            ]
        );
    }

    #[test]
    fn proof_menu_items_follow_submenu_context() {
        let items = proof_menu_items(
            &build_tray_menu(&sample_provider_catalog()),
            "tray/providers",
        )
        .unwrap();

        assert_eq!(items, vec!["Open Codex", "Open Claude"]);
    }

    #[test]
    fn proof_menu_context_for_leaf_item_returns_parent_menu() {
        let (menu_path, items) =
            proof_menu_context_for_item(&build_tray_menu(&sample_provider_catalog()), "about")
                .unwrap();

        assert_eq!(menu_path, "tray");
        assert!(items.iter().any(|item| item == "About"));
    }
}
