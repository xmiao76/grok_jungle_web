//! Single source for release identity. Title, filename, `info()`, the menu,
//! and the README are all derived from these constants.

pub const MODEL: &str = "Grok-4.7";
pub const CODE_AGENT: &str = "GrokBuild";
pub const MODEL_PLATFORM: &str = "official";
pub const GITHUB_URL: &str = "https://github.com/xmiao76/grok_jungle_web";
pub const GAME_VERSION: &str = "2.0.0";
pub const ENGINE_VERSION: &str = "2.0.0";
pub const RUNTIME: &str = "Rust";
pub const RULES_ID: &str = "desktop-jungle-v2";
pub const RULES_DIGEST: &str =
    "a457bbc009e975a9eafe5ff1f8fa40ca2548f2a689b121c8c4ed99dd0f3fdc57";

/// `<model> · <code-agent>`, plus ` · <platform>` only when the platform is
/// non-empty and not `official`.
pub fn title() -> String {
    if MODEL_PLATFORM.is_empty() || MODEL_PLATFORM == "official" {
        format!("{MODEL} · {CODE_AGENT}")
    } else {
        format!("{MODEL} · {CODE_AGENT} · {MODEL_PLATFORM}")
    }
}

/// Lowercase, collapse every run of non-alphanumeric characters to one hyphen,
/// and trim hyphens. Dots in the version are preserved by the caller.
pub fn slug_part(text: &str) -> String {
    let mut out = String::new();
    let mut hyphen = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            hyphen = false;
        } else if !hyphen && !out.is_empty() {
            out.push('-');
            hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn wasm_filename() -> String {
    let mut name = format!("{}-{}", slug_part(MODEL), slug_part(CODE_AGENT));
    if !MODEL_PLATFORM.is_empty() && MODEL_PLATFORM != "official" {
        name.push('-');
        name.push_str(&slug_part(MODEL_PLATFORM));
    }
    format!("{name}-{GAME_VERSION}.wasm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_omits_official_platform() {
        assert_eq!(title(), "Grok-4.7 · GrokBuild");
    }

    #[test]
    fn filename_omits_official_platform() {
        assert_eq!(wasm_filename(), "grok-4-7-grokbuild-2.0.0.wasm");
    }

    #[test]
    fn slug_collapses_runs() {
        assert_eq!(slug_part("Claude-Opus-5.5"), "claude-opus-5-5");
        assert_eq!(slug_part("--A--B--"), "a-b");
    }
}
