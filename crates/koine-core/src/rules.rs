//! Built-in versioned rule identities; no I/O or external format parser.
use crate::{canonical, *};
pub const STEAM_INTERPRETATION: &str = "koine.steam.manifest.v1";
pub const STEAM_CONTROLLED: &str = "koine.steam.controlled.v2";
pub const STEAM_IDENTITY: &str = "koine.steam.app-id.v1";
pub const RECONCILIATION: &str = "koine.evidence.v1";
pub const FRESHNESS: &str = "koine.source-freshness.v1";
pub fn version(name: &str) -> &'static str {
    if name == STEAM_CONTROLLED { "2" } else { "1" }
}
pub fn artifact(name: &str, kind: &str) -> Option<Vec<u8>> {
    let descriptor = match (name, kind) {
        (STEAM_INTERPRETATION, "interpretation") => {
            "koine.steam.manifest.v1:bounded-root;reported-presence;explicit-completion;opaque-revision;no-negative;no-entitlement;no-handoff;source-age-unknown"
        }
        (STEAM_CONTROLLED, "interpretation") => {
            "koine.steam.controlled.v2:explicit-pinned-roots;no-auto-expansion;openat2-no-follow-no-xdev;manifest-v1-fields;native-flatpak-explicit-library;no-negative;no-entitlement;no-handoff;opaque-revision;source-age-unknown"
        }
        (STEAM_IDENTITY, "identity") => {
            "koine.steam.app-id.v1:nonzero-decimal-u32;global-steam:app;preserve-reported-key"
        }
        (RECONCILIATION, "reconciliation") => {
            "koine.evidence.v1:scoped-proof-required;no-arrival-order;unordered-conflict-disputed;immutable-seals"
        }
        (FRESHNESS, "freshness") => {
            "koine.source-freshness.v1:explicit-assessment-time;preserve-source-time;unknown-source-age-not-renewed"
        }
        _ => return None,
    };
    Some(canonical::hash(descriptor.as_bytes()))
}
pub fn steam_key(text: &str) -> Result<String> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid Steam application ID".into());
    }
    let value: u32 = text
        .parse()
        .map_err(|_| "Steam application ID outside u32")?;
    if value == 0 {
        return Err("Steam application ID is zero".into());
    }
    Ok(value.to_string())
}
