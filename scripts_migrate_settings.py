#!/usr/bin/env python3
"""One-shot migration script for Phase 3 settings refactor.

Rewrites read/write call sites of the legacy flat per-provider Settings
fields to use the new accessor methods (which return owned-or-borrowed
values and named setter methods).
"""
import re
import sys
from pathlib import Path

FIELDS = [
    "codex_cookie_source",
    "claude_cookie_source",
    "cursor_cookie_source",
    "opencode_cookie_source",
    "factory_cookie_source",
    "alibaba_cookie_source",
    "kimi_cookie_source",
    "minimax_cookie_source",
    "augment_cookie_source",
    "amp_cookie_source",
    "ollama_cookie_source",
    "claude_usage_source",
    "codex_usage_source",
    "alibaba_api_region",
    "zai_api_region",
    "minimax_api_region",
    "alibaba_cookie_header",
    "kimi_manual_cookie_header",
    "augment_cookie_header",
    "amp_cookie_header",
    "ollama_cookie_header",
    "minimax_cookie_header",
    "opencode_workspace_id",
    "minimax_api_token",
    "jetbrains_ide_base_path",
    "codex_openai_web_extras",
    "codex_historical_tracking",
    "claude_avoid_keychain_prompts",
]

# Bool-typed fields: setter takes bool, reader returns bool (no clone/to_string).
BOOL_FIELDS = {
    "codex_openai_web_extras",
    "codex_historical_tracking",
    "claude_avoid_keychain_prompts",
}


def migrate(src: str) -> str:
    out = src
    for field in FIELDS:
        # Match an owner expression followed by `.<field>` then a token that
        # tells us whether this is an assignment or a value position.
        # We require that the next char is NOT `(` (already migrated) and not
        # an identifier char (so we don't match longer field names).

        # 1) Assignment: `<expr>.<field> = <rhs>;` -> `<expr>.set_<field>(<rhs>);`
        #    Use a negative lookbehind to avoid matching `==`, `!=`, `>=`, `<=`.
        assign_pat = re.compile(
            r"([A-Za-z_][A-Za-z0-9_\.\(\)\[\]\*&\s]*?)\.(" + field + r")\s*=(?!=)\s*([^;]+);"
        )

        def repl_assign(m):
            owner = m.group(1).rstrip()
            rhs = m.group(3).strip()
            # If rhs ends with .clone() and field is a String type, collapse.
            return f"{owner}.set_{field}({rhs});"

        out = assign_pat.sub(repl_assign, out)

        # 2) Read with .clone(): `.<field>.clone()` -> `.<field>().to_string()` (or `()` for bool)
        if field in BOOL_FIELDS:
            out = re.sub(r"\.(" + field + r")\.clone\(\)", r".\1()", out)
            out = re.sub(r"\.(" + field + r")(?=[^A-Za-z0-9_\(])", r".\1()", out)
        else:
            out = re.sub(r"\.(" + field + r")\.clone\(\)", r".\1().to_string()", out)
            out = re.sub(r"\.(" + field + r")\.trim\(\)", r".\1().trim()", out)
            out = re.sub(r"\.(" + field + r")\.is_empty\(\)", r".\1().is_empty()", out)
            # bare reads not followed by ident/( char
            out = re.sub(r"\.(" + field + r")(?=[^A-Za-z0-9_\(])", r".\1()", out)
    return out


def main():
    root = Path(__file__).resolve().parent
    targets = [
        root / "rust" / "legacy" / "native_ui" / "preferences.rs",
        root / "apps" / "desktop-tauri" / "src-tauri" / "src" / "commands" / "mod.rs",
    ]
    for path in targets:
        text = path.read_text()
        new = migrate(text)
        if new != text:
            path.write_text(new)
            print(f"updated {path}")
        else:
            print(f"unchanged {path}")


if __name__ == "__main__":
    main()
