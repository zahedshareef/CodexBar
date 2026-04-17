# Pending ZH (Chinese) locale review — Phases 5–12

This file is the single point of batch review for all Chinese (`zh-CN`)
translations introduced during the egui→Tauri port (Phases 5–12) that
carry a `REVIEW-i18n` marker in `rust/src/locale.rs`, plus the Phase-6e
token-account strings that were landed as a self-flagged review batch.

**How to use**

1. Walk the table top-to-bottom.
2. If the current ZH wording is acceptable, copy it verbatim into the
   `Reviewed ZH` column.
3. If it needs to change, put the improved wording in `Reviewed ZH` and
   update `rust/src/locale.rs` to match before unflagging.
4. Once every row has a `Reviewed ZH` value, delete the corresponding
   `// REVIEW-i18n` comments from `rust/src/locale.rs` and this doc.

The `{}` placeholders are `std::fmt::Display` substitution points and
must be preserved exactly (including count and order).

---

## Phase 7 — Shortcut capture + notification test labels
Source: `rust/src/locale.rs:1137` (EN) and `1774` (ZH)

| LocaleKey                        | English                                                   | Current ZH                                     | Reviewed ZH |
| -------------------------------- | --------------------------------------------------------- | ---------------------------------------------- | ----------- |
| ShortcutRecordButton             | Record                                                    | 录制                                           |             |
| ShortcutRecordingLabel           | Recording…                                                | 录制中…                                        |             |
| ShortcutRecordingHint            | Press modifiers + a key. Esc cancels, Backspace clears.   | 按下修饰键 + 任意键。Esc 取消，Backspace 清除。 |             |
| ShortcutClearButton              | Clear                                                     | 清除                                           |             |
| ShortcutEmptyPlaceholder         | Not set                                                   | 未设置                                         |             |
| NotificationTestSound            | Test sound                                                | 测试声音                                       |             |
| NotificationTestSoundPlaying     | Playing…                                                  | 播放中…                                        |             |

## Phase 9 — Tray / pop-out pace badges + reset countdowns
Source: `rust/src/locale.rs:1339` (EN) and `1963` (ZH)

| LocaleKey            | English        | Current ZH  | Reviewed ZH |
| -------------------- | -------------- | ----------- | ----------- |
| TrayPaceBadgeSlow    | Slow           | 缓慢        |             |
| TrayPaceBadgeSteady  | Steady         | 稳定        |             |
| TrayPaceBadgeRacing  | Racing         | 加速        |             |
| TrayPaceBadgeBurning | Burning        | 超速        |             |
| TrayResetsInLabel    | Resets in {}   | {} 后重置   |             |
| TrayResetsDueNow     | Resetting…     | 正在重置…   |             |

## Phase 10 — Detail chart empty state
Source: `rust/src/locale.rs:1255` (EN) and `1881` (ZH)

| LocaleKey        | English             | Current ZH       | Reviewed ZH |
| ---------------- | ------------------- | ---------------- | ----------- |
| DetailChartEmpty | No chart data yet.  | 暂无图表数据。   |             |

## Phase 12 — Theme (appearance) toggle
Source: `rust/src/locale.rs:1198` (EN) and `1826` (ZH)

| LocaleKey         | English                                                              | Current ZH                                   | Reviewed ZH |
| ----------------- | -------------------------------------------------------------------- | -------------------------------------------- | ----------- |
| SectionTheme      | Appearance                                                           | 外观                                         |             |
| ThemeLabel        | Theme                                                                | 主题                                         |             |
| ThemeHelper       | Auto follows your system color scheme. Light and Dark override it.   | 自动跟随系统配色方案；浅色/深色可手动覆盖。  |             |
| ThemeAutoOption   | Auto (system)                                                        | 自动（跟随系统）                             |             |
| ThemeLightOption  | Light                                                                | 浅色                                         |             |
| ThemeDarkOption   | Dark                                                                 | 深色                                         |             |

## Phase 6e — Token accounts (self-flagged as "review" in-line)
Source: `rust/src/locale.rs:1322` (EN) and `1945` (ZH)

| LocaleKey                        | English                                                                                                                                               | Current ZH                                                                                                    | Reviewed ZH |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------- |
| TokenAccountActive               | Active                                                                                                                                                | 活动                                                                                                          |             |
| TokenAccountSetActive            | Set Active                                                                                                                                            | 设为活动                                                                                                      |             |
| TokenAccountRemove               | Remove                                                                                                                                                | 移除                                                                                                          |             |
| TokenAccountAddButton            | Add Account                                                                                                                                           | 添加账户                                                                                                      |             |
| TokenAccountEmpty                | No accounts saved for this provider.                                                                                                                  | 该服务商尚未保存任何账户。                                                                                    |             |
| TokenAccountLabelPlaceholder     | Label (e.g. Work, Personal)…                                                                                                                          | 标签（如工作、个人）…                                                                                         |             |
| TokenAccountProviderLabel        | Provider                                                                                                                                              | 服务商                                                                                                        |             |
| TokenAccountProviderPlaceholder  | Select provider…                                                                                                                                      | 选择服务商…                                                                                                   |             |
| TokenAccountAddedPrefix          | Added                                                                                                                                                 | 添加于                                                                                                        |             |
| TokenAccountUsedPrefix           | Used                                                                                                                                                  | 上次使用                                                                                                      |             |
| TokenAccountTabHint              | Manage multiple session tokens or API tokens per provider. The active account is used for all fetches. Only providers that require manual tokens appear here. | 按服务商管理多个会话令牌或 API 令牌。所有数据拉取都会使用活动账户。仅需要手动令牌的服务商会显示在此处。 |             |
| TokenAccountNoSupported          | No providers currently support token accounts.                                                                                                        | 当前没有支持令牌账户的服务商。                                                                                |             |
| TokenAccountInlineSummary        | Token accounts                                                                                                                                        | 令牌账户                                                                                                      |             |

---

**Summary:** 33 keys pending ZH review across Phases 5–12
(7 Phase 7 + 6 Phase 9 + 1 Phase 10 + 6 Phase 12 + 13 Phase 6e).
