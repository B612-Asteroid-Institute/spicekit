//! Minimal text-kernel parser for NAIF body name/code bindings.
//!
//! Text kernels (`.tk`, `.tf`, `.tpc`, `.ti`) alternate `\begintext` /
//! `\begindata` blocks; only data blocks carry assignments. This parser
//! is deliberately scoped: inside data blocks it extracts
//!
//! ```text
//!   NAIF_BODY_NAME += ( 'JWST', 'JAMES WEBB SPACE TELESCOPE' )
//!   NAIF_BODY_CODE += ( -170, -170 )
//! ```
//!
//! and ignores every other assignment. Both `=` and `+=` are treated
//! as providing additional bindings — CSpice semantics are
//! last-loaded-wins, so the caller layers these on top of earlier
//! entries. Scalars and parenthesised lists are both accepted. String
//! values may be single- or double-quoted; embedded `''` / `""` is a
//! literal quote.
//!
//! By only recognising two specific keys we stay robust to the full
//! diversity of text-kernel content (LSK deltas with Fortran `D-`
//! notation, SCLK partitions, date literals like `@1972-JAN-1`, frame
//! definitions, etc.). Anything we don't need is skipped character-by-
//! character until we resync on a `\begintext` marker or a new
//! `NAIF_BODY_*` assignment.

use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum TextKernelError {
    #[error("io error reading text kernel {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error(
        "NAIF_BODY_NAME has {names} entries but NAIF_BODY_CODE has {codes} — they must match"
    )]
    Mismatched { names: usize, codes: usize },
}

/// A single (name, code) binding parsed from a text kernel, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyBinding {
    pub name: String,
    pub code: i32,
}

/// Parse a text kernel file and return the ordered list of
/// `NAIF_BODY_NAME` ↔ `NAIF_BODY_CODE` bindings.
///
/// File-order is preserved so downstream callers can apply
/// last-loaded-wins semantics to match CSpice.
pub fn parse_body_bindings(path: &Path) -> Result<Vec<BodyBinding>, TextKernelError> {
    let text = fs::read_to_string(path).map_err(|e| TextKernelError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_body_bindings_from_str(&text)
}

/// Parse an in-memory text-kernel string.
pub fn parse_body_bindings_from_str(text: &str) -> Result<Vec<BodyBinding>, TextKernelError> {
    let mut names: Vec<String> = Vec::new();
    let mut codes: Vec<i32> = Vec::new();

    let mut in_data = false;
    let mut line_no = 1usize;
    // We sweep the text once, tracking line numbers. Within data
    // blocks we look for NAIF_BODY_NAME / NAIF_BODY_CODE and harvest
    // their RHS values; everything else inside a data block is
    // skipped.
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Check for line-anchored directives.
        if bytes[i] == b'\\' {
            if let Some(word_end) = directive_end(bytes, i) {
                let word = &text[i..word_end];
                if word.eq_ignore_ascii_case("\\begindata") {
                    in_data = true;
                    i = word_end;
                    continue;
                } else if word.eq_ignore_ascii_case("\\begintext") {
                    in_data = false;
                    i = word_end;
                    continue;
                }
            }
        }

        if !in_data {
            if bytes[i] == b'\n' {
                line_no += 1;
            }
            i += 1;
            continue;
        }

        // Inside a data block. Look for a NAIF_BODY_NAME or
        // NAIF_BODY_CODE assignment. A real assignment starts with
        // either of those identifiers at a token boundary.
        if is_ident_start(bytes[i]) && is_word_boundary_before(bytes, i) {
            let (end, ident) = read_identifier(text, i);
            if ident == "NAIF_BODY_NAME" || ident == "NAIF_BODY_CODE" {
                let (after_op, _op_line_delta) = skip_equals(bytes, end)?;
                line_no += count_newlines(&bytes[end..after_op]);
                let (after_values, values, delta) =
                    read_values(text, after_op).map_err(|msg| TextKernelError::Parse {
                        line: line_no,
                        message: msg,
                    })?;
                line_no += delta;
                if ident == "NAIF_BODY_NAME" {
                    for v in values {
                        match v {
                            Value::String(s) => names.push(s),
                            _ => {
                                return Err(TextKernelError::Parse {
                                    line: line_no,
                                    message: "NAIF_BODY_NAME expects strings".into(),
                                })
                            }
                        }
                    }
                } else {
                    for v in values {
                        match v {
                            Value::Int(n) => codes.push(n),
                            _ => {
                                return Err(TextKernelError::Parse {
                                    line: line_no,
                                    message: "NAIF_BODY_CODE expects integers".into(),
                                })
                            }
                        }
                    }
                }
                i = after_values;
                continue;
            } else {
                // Not one we care about — advance past the identifier
                // and let the loop continue scanning.
                i = end;
                continue;
            }
        }

        if bytes[i] == b'\n' {
            line_no += 1;
        }
        i += 1;
    }

    if names.len() != codes.len() {
        return Err(TextKernelError::Mismatched {
            names: names.len(),
            codes: codes.len(),
        });
    }

    Ok(names
        .into_iter()
        .zip(codes)
        .map(|(name, code)| BodyBinding { name, code })
        .collect())
}

#[derive(Debug)]
enum Value {
    String(String),
    Int(i32),
    Other,
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'/'
}

fn is_word_boundary_before(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = bytes[i - 1];
    !(prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'/')
}

fn read_identifier(text: &str, start: usize) -> (usize, &str) {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() && is_ident_continue(bytes[end]) {
        end += 1;
    }
    (end, &text[start..end])
}

fn directive_end(bytes: &[u8], start: usize) -> Option<usize> {
    // Read \ + word of alphabetic chars.
    let mut end = start + 1;
    while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
        end += 1;
    }
    if end == start + 1 {
        None
    } else {
        Some(end)
    }
}

fn count_newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'\n').count()
}

/// Skip whitespace then require `=` or `+=`; return the index after
/// the operator and the number of newlines consumed.
fn skip_equals(bytes: &[u8], mut i: usize) -> Result<(usize, usize), TextKernelError> {
    let mut delta = 0usize;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'=' {
        i += 1;
        Ok((i, delta))
    } else {
        // Allow newline between key and operator (rare but legal).
        while i < bytes.len()
            && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n')
        {
            if bytes[i] == b'\n' {
                delta += 1;
            }
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            Ok((i, delta))
        } else {
            Err(TextKernelError::Parse {
                line: 0,
                message: "expected '=' or '+=' after NAIF_BODY_NAME/CODE".into(),
            })
        }
    }
}

/// Read a scalar or parenthesised list of values starting at `i`.
/// Returns (next_index, values, newlines_consumed).
fn read_values(
    text: &str,
    mut i: usize,
) -> Result<(usize, Vec<Value>, usize), String> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut delta = 0usize;

    // Skip leading whitespace (including newlines before the value).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        if bytes[i] == b'\n' {
            delta += 1;
        }
        i += 1;
    }

    let parenthesised = i < bytes.len() && bytes[i] == b'(';
    if parenthesised {
        i += 1;
    }

    loop {
        // Skip whitespace and commas between elements.
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '\n' {
                delta += 1;
                i += 1;
            } else if c.is_whitespace() || c == ',' {
                i += 1;
            } else {
                break;
            }
        }
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b')' if parenthesised => {
                i += 1;
                break;
            }
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                let mut s = String::new();
                while i < bytes.len() {
                    if bytes[i] == quote {
                        // Doubled quote = escaped quote.
                        if i + 1 < bytes.len() && bytes[i + 1] == quote {
                            s.push(quote as char);
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    if bytes[i] == b'\n' {
                        delta += 1;
                    }
                    s.push(bytes[i] as char);
                    i += 1;
                }
                values.push(Value::String(s));
            }
            b if b == b'-' || b == b'+' || b.is_ascii_digit() || b == b'.' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    let c = bytes[i];
                    if c.is_ascii_digit()
                        || c == b'.'
                        || c == b'e'
                        || c == b'E'
                        || c == b'd'
                        || c == b'D'
                        || c == b'+'
                        || c == b'-'
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let tok = &text[start..i];
                if let Ok(n) = tok.parse::<i32>() {
                    values.push(Value::Int(n));
                } else {
                    // Non-integer numeric (e.g. 32.184, 1.657D-3). Not
                    // a body code; record as Other so caller errors
                    // out cleanly if mis-typed.
                    values.push(Value::Other);
                }
            }
            b'@' => {
                // Date literal like @1972-JAN-1. Skip it.
                i += 1;
                while i < bytes.len() {
                    let c = bytes[i];
                    if c.is_ascii_alphanumeric() || c == b'-' || c == b':' || c == b'.' || c == b'/' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                values.push(Value::Other);
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                // Bare identifier / symbol (e.g. logical TRUE). Skip.
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
                {
                    i += 1;
                }
                values.push(Value::Other);
            }
            _ => break,
        }
        if !parenthesised {
            break;
        }
    }

    Ok((i, values, delta))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_jwst_block() {
        let kernel = "KPL/FK\n\n\\begindata\n  NAIF_BODY_NAME += ( 'JWST', 'JAMES WEBB SPACE TELESCOPE' )\n  NAIF_BODY_CODE += ( -170, -170 )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0], BodyBinding { name: "JWST".into(), code: -170 });
        assert_eq!(
            bindings[1],
            BodyBinding {
                name: "JAMES WEBB SPACE TELESCOPE".into(),
                code: -170,
            }
        );
    }

    #[test]
    fn ignores_comment_sections() {
        let kernel = "Free text preamble.\n   NAIF_BODY_NAME += ( 'IGNORED' )\n   NAIF_BODY_CODE += ( 999 )\n\n\\begindata\n  NAIF_BODY_NAME += ( 'CUSTOM' )\n  NAIF_BODY_CODE += ( -999 )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings, vec![BodyBinding { name: "CUSTOM".into(), code: -999 }]);
    }

    #[test]
    fn supports_scalar_and_equals() {
        let kernel = "\\begindata\n  NAIF_BODY_NAME = 'LONE_BODY'\n  NAIF_BODY_CODE = -42\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(
            bindings,
            vec![BodyBinding { name: "LONE_BODY".into(), code: -42 }]
        );
    }

    #[test]
    fn concatenates_multiple_data_blocks() {
        let kernel = "\\begindata\n  NAIF_BODY_NAME += ( 'A' )\n  NAIF_BODY_CODE += ( 1 )\n\\begintext\nfiller\n\\begindata\n  NAIF_BODY_NAME += ( 'B' )\n  NAIF_BODY_CODE += ( 2 )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(
            bindings,
            vec![
                BodyBinding { name: "A".into(), code: 1 },
                BodyBinding { name: "B".into(), code: 2 },
            ]
        );
    }

    #[test]
    fn rejects_mismatched_arrays() {
        let kernel = "\\begindata\n  NAIF_BODY_NAME += ( 'A', 'B' )\n  NAIF_BODY_CODE += ( 1 )\n\\begintext\n";
        let err = parse_body_bindings_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Mismatched { names: 2, codes: 1 }));
    }

    #[test]
    fn handles_double_quotes_and_line_continuations() {
        let kernel = "\\begindata\n   NAIF_BODY_NAME += (\n     \"ALPHA\",\n     \"BETA\"\n   )\n   NAIF_BODY_CODE += (\n     100,\n     200\n   )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].name, "ALPHA");
        assert_eq!(bindings[1].code, 200);
    }

    #[test]
    fn ignores_unrelated_assignments() {
        let kernel = "\\begindata\n  OBJECT_EARTH_FRAME = 'ITRF93'\n  NAIF_BODY_NAME += ( 'ONE' )\n  NAIF_BODY_CODE += ( 1 )\n  DELTET/DELTA_T_A = 32.184\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings, vec![BodyBinding { name: "ONE".into(), code: 1 }]);
    }

    #[test]
    fn no_data_block_returns_empty() {
        let bindings = parse_body_bindings_from_str("just a comment file").unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn tolerates_leapseconds_kernel_content() {
        // Distilled from latest_leapseconds.tls — the actual file
        // trips a naive parser because of `1.657D-3` (Fortran double
        // notation), `@1972-JAN-1` date literals, and slash-containing
        // keys like `DELTET/K`.
        let kernel = r#"
\begindata

DELTET/DELTA_T_A       =   32.184
DELTET/K               =    1.657D-3
DELTET/EB              =    1.671D-2
DELTET/M               = (  6.239996D0   1.99096871D-7 )

DELTET/DELTA_AT        = ( 10,   @1972-JAN-1
                           11,   @1972-JUL-1
                           12,   @1973-JAN-1 )

NAIF_BODY_NAME += ( 'EXTRA_BODY' )
NAIF_BODY_CODE += ( -12345 )

\begintext
"#;
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings, vec![BodyBinding { name: "EXTRA_BODY".into(), code: -12345 }]);
    }
}
