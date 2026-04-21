//! NAIF built-in body name ↔ integer ID lookup.
//!
//! Mirrors CSpice's built-in body-code table exactly. The source data is
//! extracted from `zzidmap.c` (the NAIF toolkit's generated name/ID map)
//! and stored in `naif_builtin_table.rs`. Names are matched
//! case-insensitively with whitespace/underscore collapsing to mirror
//! CSpice's `bodn2c` behaviour.
//!
//! Custom or mission-specific name bindings (e.g. user-supplied aliases
//! loaded via `NAIF_BODY_NAME +=` / `NAIF_BODY_CODE +=` in a text kernel)
//! are NOT handled here — they come from the text-kernel parser and the
//! per-backend body-name registry, which takes precedence over this
//! table (last-loaded-wins, matching CSpice).
//!
//! Reference: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/naif_ids.html

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, thiserror::Error)]
pub enum NaifIdError {
    #[error("NAIF body name '{0}' is not in the built-in table")]
    UnknownName(String),
    #[error("NAIF body code {0} is not in the built-in table")]
    UnknownCode(i32),
}

// Full 692-entry built-in table, generated from CSpice's zzidmap.c.
// For each NAIF code, the first alias listed is the one CSpice's
// `bodc2n` returns as canonical — `bodc2n` below returns the first
// occurrence so the canonical-ordering is load-bearing for parity.
include!("naif_builtin_table.rs");

static NAME_TO_CODE: OnceLock<HashMap<String, i32>> = OnceLock::new();
static CODE_TO_NAME: OnceLock<HashMap<i32, String>> = OnceLock::new();

fn name_to_code() -> &'static HashMap<String, i32> {
    NAME_TO_CODE.get_or_init(|| {
        let mut m = HashMap::with_capacity(BUILTIN.len());
        for &(name, code) in BUILTIN {
            m.insert(normalize_name(name), code);
        }
        m
    })
}

fn code_to_name() -> &'static HashMap<i32, String> {
    CODE_TO_NAME.get_or_init(|| {
        // For bodc2n we want the *canonical* (first) spelling per code,
        // matching CSpice's behavior where the first name in the table
        // wins for reverse lookups.
        let mut m: HashMap<i32, String> = HashMap::new();
        for &(name, code) in BUILTIN {
            m.entry(code).or_insert_with(|| name.to_string());
        }
        m
    })
}

/// Normalize a body name to match CSpice's `bodn2c` matching: collapse
/// internal whitespace runs to a single space, trim leading/trailing
/// whitespace, uppercase. Underscores are treated as spaces so callers
/// can pass `OriginCodes` names directly (`EARTH_MOON_BARYCENTER`).
pub(crate) fn normalize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_space = true;
    for ch in name.chars() {
        if ch.is_whitespace() || ch == '_' {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            for up in ch.to_uppercase() {
                out.push(up);
            }
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Look up a body NAIF ID by name against the built-in table only.
/// Kernel-supplied bindings must be consulted separately (and before
/// this) by the caller.
pub fn bodn2c(name: &str) -> Result<i32, NaifIdError> {
    let key = normalize_name(name);
    name_to_code()
        .get(&key)
        .copied()
        .ok_or_else(|| NaifIdError::UnknownName(name.to_string()))
}

/// Reverse: body NAIF ID → canonical name from the built-in table.
pub fn bodc2n(code: i32) -> Result<&'static str, NaifIdError> {
    code_to_name()
        .get(&code)
        .map(|s| s.as_str())
        .ok_or(NaifIdError::UnknownCode(code))
}

/// Number of entries in the built-in table. Exposed for diagnostics/tests.
pub fn builtin_len() -> usize {
    BUILTIN.len()
}

/// Read-only view of the built-in (name, code) table. Exposed for the
/// spicekit-bench CSpice-parity suite to iterate every code.
pub fn builtin_entries() -> &'static [(&'static str, i32)] {
    BUILTIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_size_matches_cspice() {
        // CSpice's NPERM is 692 as of the vendored toolkit.
        assert_eq!(BUILTIN.len(), 692);
    }

    #[test]
    fn lookup_builtin_names() {
        assert_eq!(bodn2c("EARTH").unwrap(), 399);
        assert_eq!(bodn2c("earth").unwrap(), 399);
        assert_eq!(bodn2c("Earth").unwrap(), 399);
        assert_eq!(bodn2c("SUN").unwrap(), 10);
        assert_eq!(bodn2c("MOON").unwrap(), 301);
    }

    #[test]
    fn origin_codes_names_work() {
        assert_eq!(bodn2c("EARTH_MOON_BARYCENTER").unwrap(), 3);
        assert_eq!(bodn2c("SOLAR_SYSTEM_BARYCENTER").unwrap(), 0);
        assert_eq!(bodn2c("JUPITER_BARYCENTER").unwrap(), 5);
    }

    #[test]
    fn spaces_and_underscores_treated_alike() {
        assert_eq!(bodn2c("EARTH MOON BARYCENTER").unwrap(), 3);
        assert_eq!(bodn2c("EARTH_MOON_BARYCENTER").unwrap(), 3);
        assert_eq!(bodn2c(" earth  moon   barycenter ").unwrap(), 3);
    }

    #[test]
    fn spacecraft_resolve_from_builtin_table() {
        // These mirror CSpice's zzidmap.c — the test workflow that loads
        // the Horizons JWST .bsp relies on the built-in name binding
        // since the .bsp itself contains no name metadata.
        assert_eq!(bodn2c("JWST").unwrap(), -170);
        assert_eq!(bodn2c("JAMES WEBB SPACE TELESCOPE").unwrap(), -170);
        assert_eq!(bodn2c("HST").unwrap(), -48);
        assert_eq!(bodn2c("HUBBLE SPACE TELESCOPE").unwrap(), -48);
    }

    #[test]
    fn reverse_lookup_returns_cspice_canonical() {
        // These canonical names match CSpice's `bodc2n` bit-for-bit. The
        // `bodc2n_parity_all_builtin_codes` test in spicekit-bench
        // enforces parity across every code in the table.
        assert_eq!(bodc2n(0).unwrap(), "SOLAR SYSTEM BARYCENTER");
        assert_eq!(bodc2n(4).unwrap(), "MARS BARYCENTER");
        assert_eq!(bodc2n(399).unwrap(), "EARTH");
        assert_eq!(bodc2n(-170).unwrap(), "JAMES WEBB SPACE TELESCOPE");
        assert_eq!(bodc2n(-48).unwrap(), "HST");
    }

    #[test]
    fn reverse_lookup_unknown_code() {
        assert!(matches!(bodc2n(424242), Err(NaifIdError::UnknownCode(_))));
    }

    #[test]
    fn unknown_name_errors() {
        assert!(matches!(
            bodn2c("DEFINITELY_NOT_A_BODY"),
            Err(NaifIdError::UnknownName(_))
        ));
    }
}
