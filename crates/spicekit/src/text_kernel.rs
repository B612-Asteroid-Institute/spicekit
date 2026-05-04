//! Minimal text-kernel parser for the supported NAIF kernel-pool subset.
//!
//! Text kernels (`.tk`, `.tf`, `.tpc`, `.ti`, `.tls`) alternate
//! `\begintext` / `\begindata` blocks; only data blocks carry
//! assignments. This module deliberately supports the pieces spicekit
//! consumes today:
//!
//! * `NAIF_BODY_NAME` / `NAIF_BODY_CODE` body-name bindings,
//! * leapseconds-kernel `DELTET/*` values, and
//! * the narrow `OBJECT_EARTH_FRAME = 'ITRF93'` body-fixed-frame
//!   association used by the Earth→ITRF93 FK that accompanies binary
//!   Earth PCKs.
//!
//! Unsupported assignments are skipped by the parser on a best-effort
//! lexical basis; only supported assignments are semantically validated.
//! Callers can distinguish an otherwise-empty text kernel from one
//! containing supported content via [`TextKernelContent`]. Furnsh-style
//! dispatch should fail loudly for empty/unsupported text kernels rather
//! than silently dropping them.

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
    #[error("NAIF_BODY_NAME has {names} entries but NAIF_BODY_CODE has {codes} — they must match")]
    Mismatched { names: usize, codes: usize },
}

/// Supported content parsed from a text kernel.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextKernelContent {
    /// `NAIF_BODY_NAME` ↔ `NAIF_BODY_CODE` bindings in file order.
    pub bindings: Vec<BodyBinding>,
    /// Structured leapseconds-kernel content when a complete `DELTET/*`
    /// block is present.
    pub leapseconds: Option<LeapSecondsKernel>,
    /// Supported body-fixed frame associations such as
    /// `OBJECT_EARTH_FRAME = 'ITRF93'`.
    pub frame_associations: Vec<FrameAssociation>,
}

impl TextKernelContent {
    /// Returns true when the parser found no supported content.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty() && self.leapseconds.is_none() && self.frame_associations.is_empty()
    }
}

/// A single (name, code) binding parsed from a text kernel, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyBinding {
    pub name: String,
    pub code: i32,
}

/// A narrow body-to-fixed-frame association parsed from the supported
/// Earth→ITRF93 FK assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameAssociation {
    /// Body token from the variable name, for example `EARTH` in
    /// `OBJECT_EARTH_FRAME`.
    pub body: String,
    /// Frame name assigned to the body, for example `ITRF93`.
    pub frame: String,
}

/// Structured contents of a `KPL/LSK` leapseconds kernel.
#[derive(Debug, Clone, PartialEq)]
pub struct LeapSecondsKernel {
    pub delta_t_a: f64,
    pub k: f64,
    pub eb: f64,
    pub m: [f64; 2],
    pub delta_at: Vec<DeltaAtEntry>,
}

/// One `DELTET/DELTA_AT` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaAtEntry {
    pub leap_seconds: i32,
    pub date: KernelDate,
}

/// Raw SPICE kernel date literal parsed from `@YYYY-MMM-D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// Parse a text kernel file and return all supported content.
pub fn parse_text_kernel(path: &Path) -> Result<TextKernelContent, TextKernelError> {
    let text = fs::read_to_string(path).map_err(|e| TextKernelError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_text_kernel_from_str(&text)
}

/// Parse an in-memory text-kernel string and return all supported content.
pub fn parse_text_kernel_from_str(text: &str) -> Result<TextKernelContent, TextKernelError> {
    let mut names: Vec<String> = Vec::new();
    let mut codes: Vec<i32> = Vec::new();
    let mut lsk = LskBuilder::default();
    let mut frame_associations: Vec<FrameAssociation> = Vec::new();

    let mut in_data = false;
    let mut line_no = 1usize;
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if let Some(word_end) = directive_end(bytes, i) {
                let word = &text[i..word_end];
                if word.eq_ignore_ascii_case("\\begindata") {
                    in_data = true;
                    i = word_end;
                    continue;
                }
                if word.eq_ignore_ascii_case("\\begintext") {
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

        if is_ident_start(bytes[i]) && is_word_boundary_before(bytes, i) {
            let (end, ident) = read_identifier(text, i);
            let key = ident.to_ascii_uppercase();
            if is_supported_key(&key) {
                let (after_op, op_delta) =
                    skip_assignment_operator(bytes, end).map_err(|msg| TextKernelError::Parse {
                        line: line_no,
                        message: format!("{msg} after {key}"),
                    })?;
                line_no += op_delta;

                let (after_values, values, value_delta) =
                    read_values(text, after_op).map_err(|msg| TextKernelError::Parse {
                        line: line_no,
                        message: msg,
                    })?;
                apply_assignment(
                    &key,
                    values,
                    line_no,
                    &mut names,
                    &mut codes,
                    &mut lsk,
                    &mut frame_associations,
                )?;
                line_no += value_delta;
                i = after_values;
                continue;
            }

            if let Ok((after_op, op_delta)) = skip_assignment_operator(bytes, end) {
                if let Ok((after_values, _values, value_delta)) = read_values(text, after_op) {
                    line_no += op_delta + value_delta;
                    i = after_values;
                    continue;
                }
            }

            i = end;
            continue;
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

    let bindings = names
        .into_iter()
        .zip(codes)
        .map(|(name, code)| BodyBinding { name, code })
        .collect();

    Ok(TextKernelContent {
        bindings,
        leapseconds: lsk.finish()?,
        frame_associations,
    })
}

/// Parse a text kernel file and return the ordered list of
/// `NAIF_BODY_NAME` ↔ `NAIF_BODY_CODE` bindings.
///
/// File-order is preserved so callers can apply
/// last-loaded-wins semantics to match CSpice. This compatibility shim
/// uses the full supported text-kernel parser, so malformed supported
/// content such as an incomplete `DELTET/*` LSK block now returns a
/// [`TextKernelError`] instead of being silently ignored.
pub fn parse_body_bindings(path: &Path) -> Result<Vec<BodyBinding>, TextKernelError> {
    parse_text_kernel(path).map(|content| content.bindings)
}

/// Parse an in-memory text-kernel string and return body bindings only.
///
/// Like [`parse_body_bindings`], this shim validates all supported
/// assignments encountered in the text and can error on malformed LSK or
/// Earth→ITRF93 FK content even when the caller only wants body bindings.
pub fn parse_body_bindings_from_str(text: &str) -> Result<Vec<BodyBinding>, TextKernelError> {
    parse_text_kernel_from_str(text).map(|content| content.bindings)
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    String(String),
    Int(i32),
    Float(f64),
    Date(KernelDate),
    Other,
}

#[derive(Default)]
struct LskBuilder {
    first_line: Option<usize>,
    delta_t_a: Option<f64>,
    k: Option<f64>,
    eb: Option<f64>,
    m: Option<[f64; 2]>,
    delta_at: Option<Vec<DeltaAtEntry>>,
}

impl LskBuilder {
    fn mark_seen(&mut self, line: usize) {
        if self.first_line.is_none() {
            self.first_line = Some(line);
        }
    }

    fn finish(self) -> Result<Option<LeapSecondsKernel>, TextKernelError> {
        let Some(line) = self.first_line else {
            return Ok(None);
        };
        let delta_t_a = require_lsk_field(self.delta_t_a, "DELTET/DELTA_T_A", line)?;
        let k = require_lsk_field(self.k, "DELTET/K", line)?;
        let eb = require_lsk_field(self.eb, "DELTET/EB", line)?;
        let m = require_lsk_field(self.m, "DELTET/M", line)?;
        let delta_at = require_lsk_field(self.delta_at, "DELTET/DELTA_AT", line)?;
        Ok(Some(LeapSecondsKernel {
            delta_t_a,
            k,
            eb,
            m,
            delta_at,
        }))
    }
}

fn require_lsk_field<T>(field: Option<T>, name: &str, line: usize) -> Result<T, TextKernelError> {
    field.ok_or_else(|| TextKernelError::Parse {
        line,
        message: format!("incomplete leapseconds kernel: missing {name}"),
    })
}

fn is_supported_key(key: &str) -> bool {
    matches!(
        key,
        "NAIF_BODY_NAME"
            | "NAIF_BODY_CODE"
            | "DELTET/DELTA_T_A"
            | "DELTET/K"
            | "DELTET/EB"
            | "DELTET/M"
            | "DELTET/DELTA_AT"
            | "OBJECT_EARTH_FRAME"
    )
}

fn apply_assignment(
    key: &str,
    values: Vec<Value>,
    line: usize,
    names: &mut Vec<String>,
    codes: &mut Vec<i32>,
    lsk: &mut LskBuilder,
    frame_associations: &mut Vec<FrameAssociation>,
) -> Result<(), TextKernelError> {
    match key {
        "NAIF_BODY_NAME" => append_strings(key, values, line, names),
        "NAIF_BODY_CODE" => append_ints(key, values, line, codes),
        "DELTET/DELTA_T_A" => {
            lsk.mark_seen(line);
            lsk.delta_t_a = Some(expect_scalar_float(key, values, line)?);
            Ok(())
        }
        "DELTET/K" => {
            lsk.mark_seen(line);
            lsk.k = Some(expect_scalar_float(key, values, line)?);
            Ok(())
        }
        "DELTET/EB" => {
            lsk.mark_seen(line);
            lsk.eb = Some(expect_scalar_float(key, values, line)?);
            Ok(())
        }
        "DELTET/M" => {
            lsk.mark_seen(line);
            lsk.m = Some(expect_two_floats(key, values, line)?);
            Ok(())
        }
        "DELTET/DELTA_AT" => {
            lsk.mark_seen(line);
            lsk.delta_at = Some(expect_delta_at(values, line)?);
            Ok(())
        }
        "OBJECT_EARTH_FRAME" => {
            let frame = expect_scalar_string(key, values, line)?;
            if !frame.trim().eq_ignore_ascii_case("ITRF93") {
                return type_error(
                    line,
                    format!("{key} currently supports only the ITRF93 frame"),
                );
            }
            frame_associations.push(FrameAssociation {
                body: "EARTH".to_string(),
                frame: "ITRF93".to_string(),
            });
            Ok(())
        }
        _ => Ok(()),
    }
}

fn append_strings(
    key: &str,
    values: Vec<Value>,
    line: usize,
    out: &mut Vec<String>,
) -> Result<(), TextKernelError> {
    for value in values {
        match value {
            Value::String(s) => out.push(s),
            _ => return type_error(line, format!("{key} expects strings")),
        }
    }
    Ok(())
}

fn append_ints(
    key: &str,
    values: Vec<Value>,
    line: usize,
    out: &mut Vec<i32>,
) -> Result<(), TextKernelError> {
    for value in values {
        match value {
            Value::Int(n) => out.push(n),
            _ => return type_error(line, format!("{key} expects integers")),
        }
    }
    Ok(())
}

fn expect_scalar_string(
    key: &str,
    values: Vec<Value>,
    line: usize,
) -> Result<String, TextKernelError> {
    let mut iter = values.into_iter();
    let Some(value) = iter.next() else {
        return type_error(line, format!("{key} expects one string"));
    };
    if iter.next().is_some() {
        return type_error(line, format!("{key} expects one string"));
    }
    match value {
        Value::String(s) => Ok(s),
        _ => type_error(line, format!("{key} expects one string")),
    }
}

fn expect_scalar_float(key: &str, values: Vec<Value>, line: usize) -> Result<f64, TextKernelError> {
    let mut iter = values.into_iter();
    let Some(value) = iter.next() else {
        return type_error(line, format!("{key} expects one numeric value"));
    };
    if iter.next().is_some() {
        return type_error(line, format!("{key} expects one numeric value"));
    }
    value_as_f64(value).ok_or_else(|| TextKernelError::Parse {
        line,
        message: format!("{key} expects one numeric value"),
    })
}

fn expect_two_floats(
    key: &str,
    values: Vec<Value>,
    line: usize,
) -> Result<[f64; 2], TextKernelError> {
    if values.len() != 2 {
        return type_error(line, format!("{key} expects exactly two numeric values"));
    }
    let mut out = [0.0; 2];
    for (i, value) in values.into_iter().enumerate() {
        out[i] = value_as_f64(value).ok_or_else(|| TextKernelError::Parse {
            line,
            message: format!("{key} expects numeric values"),
        })?;
    }
    Ok(out)
}

fn expect_delta_at(values: Vec<Value>, line: usize) -> Result<Vec<DeltaAtEntry>, TextKernelError> {
    if values.is_empty() || values.len() % 2 != 0 {
        return type_error(
            line,
            "DELTET/DELTA_AT expects alternating integer/date pairs".to_string(),
        );
    }

    let mut entries = Vec::with_capacity(values.len() / 2);
    let mut chunks = values.into_iter();
    while let Some(leap_value) = chunks.next() {
        let Some(date_value) = chunks.next() else {
            unreachable!("odd DELTA_AT count was rejected above");
        };
        let leap_seconds = match leap_value {
            Value::Int(n) => n,
            _ => {
                return type_error(
                    line,
                    "DELTET/DELTA_AT expects integer leap-second values".to_string(),
                )
            }
        };
        let date = match date_value {
            Value::Date(date) => date,
            _ => {
                return type_error(
                    line,
                    "DELTET/DELTA_AT expects @YYYY-MMM-D date literals".to_string(),
                )
            }
        };
        entries.push(DeltaAtEntry { leap_seconds, date });
    }
    Ok(entries)
}

fn value_as_f64(value: Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(n as f64),
        Value::Float(x) => Some(x),
        _ => None,
    }
}

fn type_error<T>(line: usize, message: String) -> Result<T, TextKernelError> {
    Err(TextKernelError::Parse { line, message })
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

/// Skip whitespace, then require `=` or `+=`; return the index after
/// the operator and the number of newlines consumed.
fn skip_assignment_operator(bytes: &[u8], mut i: usize) -> Result<(usize, usize), String> {
    let mut delta = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        if bytes[i] == b'\n' {
            delta += 1;
        }
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'=' {
        return Ok((i + 1, delta));
    }
    Err("expected '=' or '+='".to_string())
}

/// Read a scalar or parenthesised list of values starting at `i`.
/// Returns (next_index, values, newlines_consumed).
fn read_values(text: &str, mut i: usize) -> Result<(usize, Vec<Value>, usize), String> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut delta = 0usize;

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
                values.push(parse_numeric_value(&text[start..i]));
            }
            b'@' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    let c = bytes[i];
                    if c.is_ascii_alphanumeric() || c == b'-' || c == b':' || c == b'.' || c == b'/'
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
                values.push(
                    parse_kernel_date_literal(&text[start..i]).map_or(Value::Other, Value::Date),
                );
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
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

fn parse_numeric_value(token: &str) -> Value {
    let is_float = token
        .as_bytes()
        .iter()
        .any(|&b| matches!(b, b'.' | b'e' | b'E' | b'd' | b'D'));
    if !is_float {
        if let Ok(n) = token.parse::<i32>() {
            return Value::Int(n);
        }
    }

    let normalized: String = token
        .chars()
        .map(|c| if c == 'D' || c == 'd' { 'E' } else { c })
        .collect();
    normalized.parse::<f64>().map_or(Value::Other, Value::Float)
}

fn parse_kernel_date_literal(token: &str) -> Option<KernelDate> {
    let rest = token.strip_prefix('@')?;
    let mut parts = rest.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parse_month(parts.next()?)?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() || !valid_day(year, month, day) {
        return None;
    }
    Some(KernelDate { year, month, day })
}

fn parse_month(raw: &str) -> Option<u8> {
    match raw.to_ascii_uppercase().as_str() {
        "JAN" => Some(1),
        "FEB" => Some(2),
        "MAR" => Some(3),
        "APR" => Some(4),
        "MAY" => Some(5),
        "JUN" => Some(6),
        "JUL" => Some(7),
        "AUG" => Some(8),
        "SEP" => Some(9),
        "OCT" => Some(10),
        "NOV" => Some(11),
        "DEC" => Some(12),
        _ => None,
    }
}

fn valid_day(year: i32, month: u8, day: u8) -> bool {
    if day == 0 {
        return false;
    }
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };
    day <= max
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lsk_kernel() -> &'static str {
        r#"
\begindata

DELTET/DELTA_T_A       =   32.184
DELTET/K               =    1.657D-3
DELTET/EB              =    1.671D-2
DELTET/M               = (  6.239996D0   1.99096871D-7 )

DELTET/DELTA_AT        = ( 10,   @1972-JAN-1
                           11,   @1972-JUL-1
                           12,   @1973-JAN-1 )

\begintext
"#
    }

    #[test]
    fn parses_simple_jwst_block() {
        let kernel = "KPL/FK\n\n\\begindata\n  NAIF_BODY_NAME += ( 'JWST', 'JAMES WEBB SPACE TELESCOPE' )\n  NAIF_BODY_CODE += ( -170, -170 )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(
            bindings[0],
            BodyBinding {
                name: "JWST".into(),
                code: -170
            }
        );
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
        assert_eq!(
            bindings,
            vec![BodyBinding {
                name: "CUSTOM".into(),
                code: -999
            }]
        );
    }

    #[test]
    fn supports_scalar_and_equals() {
        let kernel =
            "\\begindata\n  NAIF_BODY_NAME = 'LONE_BODY'\n  NAIF_BODY_CODE = -42\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(
            bindings,
            vec![BodyBinding {
                name: "LONE_BODY".into(),
                code: -42
            }]
        );
    }

    #[test]
    fn concatenates_multiple_data_blocks() {
        let kernel = "\\begindata\n  NAIF_BODY_NAME += ( 'A' )\n  NAIF_BODY_CODE += ( 1 )\n\\begintext\nfiller\n\\begindata\n  NAIF_BODY_NAME += ( 'B' )\n  NAIF_BODY_CODE += ( 2 )\n\\begintext\n";
        let bindings = parse_body_bindings_from_str(kernel).unwrap();
        assert_eq!(
            bindings,
            vec![
                BodyBinding {
                    name: "A".into(),
                    code: 1
                },
                BodyBinding {
                    name: "B".into(),
                    code: 2
                },
            ]
        );
    }

    #[test]
    fn rejects_mismatched_arrays() {
        let kernel = "\\begindata\n  NAIF_BODY_NAME += ( 'A', 'B' )\n  NAIF_BODY_CODE += ( 1 )\n\\begintext\n";
        let err = parse_body_bindings_from_str(kernel).unwrap_err();
        assert!(matches!(
            err,
            TextKernelError::Mismatched { names: 2, codes: 1 }
        ));
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
    fn ignores_unrelated_assignments_when_supported_content_exists() {
        let kernel = "\\begindata\n  FRAME_FOO = 123\n  OBJECT_MARS_FRAME = 'IAU_MARS'\n  NAIF_BODY_NAME += ( 'ONE' )\n  NAIF_BODY_CODE += ( 1 )\n\\begintext\n";
        let content = parse_text_kernel_from_str(kernel).unwrap();
        assert_eq!(
            content.bindings,
            vec![BodyBinding {
                name: "ONE".into(),
                code: 1
            }]
        );
        assert!(content.frame_associations.is_empty());
    }

    #[test]
    fn no_data_block_returns_empty() {
        let content = parse_text_kernel_from_str("just a comment file").unwrap();
        assert!(content.is_empty());
        assert!(parse_body_bindings_from_str("just a comment file")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn skips_unsupported_assignment_values_before_resuming() {
        let kernel = "\\begindata\n  DESCRIPTION = 'OBJECT_EARTH_FRAME = ITRF93'\n  NAIF_BODY_NAME += ( 'REAL' )\n  NAIF_BODY_CODE += ( 7 )\n\\begintext\n";
        let content = parse_text_kernel_from_str(kernel).unwrap();
        assert!(content.frame_associations.is_empty());
        assert_eq!(content.bindings[0].name, "REAL");
    }

    #[test]
    fn parses_leapseconds_kernel_content() {
        let content = parse_text_kernel_from_str(lsk_kernel()).unwrap();
        let lsk = content.leapseconds.unwrap();
        assert_eq!(lsk.delta_t_a, 32.184);
        assert_eq!(lsk.k, 1.657e-3);
        assert_eq!(lsk.eb, 1.671e-2);
        assert_eq!(lsk.m, [6.239996, 1.99096871e-7]);
        assert_eq!(lsk.delta_at.len(), 3);
        assert_eq!(
            lsk.delta_at[0],
            DeltaAtEntry {
                leap_seconds: 10,
                date: KernelDate {
                    year: 1972,
                    month: 1,
                    day: 1,
                }
            }
        );
        assert_eq!(lsk.delta_at[1].date.month, 7);
    }

    #[test]
    fn body_bindings_shim_tolerates_complete_lsk_content() {
        let kernel = format!(
            "{}\\begindata\nNAIF_BODY_NAME += ( 'EXTRA_BODY' )\nNAIF_BODY_CODE += ( -12345 )\n\\begintext\n",
            lsk_kernel()
        );
        let bindings = parse_body_bindings_from_str(&kernel).unwrap();
        assert_eq!(
            bindings,
            vec![BodyBinding {
                name: "EXTRA_BODY".into(),
                code: -12345
            }]
        );
    }

    #[test]
    fn parses_earth_itrf93_frame_association() {
        let kernel = "KPL/FK\n\\begindata\n  OBJECT_EARTH_FRAME = 'ITRF93'\n\\begintext\n";
        let content = parse_text_kernel_from_str(kernel).unwrap();
        assert_eq!(
            content.frame_associations,
            vec![FrameAssociation {
                body: "EARTH".into(),
                frame: "ITRF93".into(),
            }]
        );
    }

    #[test]
    fn rejects_unsupported_earth_frame_association() {
        let kernel = "KPL/FK\n\\begindata\n  OBJECT_EARTH_FRAME = 'IAU_EARTH'\n\\begintext\n";
        let err = parse_text_kernel_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Parse { .. }));
    }

    #[test]
    fn rejects_delta_at_odd_count() {
        let kernel = "\\begindata\nDELTET/DELTA_T_A=32.184\nDELTET/K=1.657D-3\nDELTET/EB=1.671D-2\nDELTET/M=(6.239996D0 1.99096871D-7)\nDELTET/DELTA_AT=(10 @1972-JAN-1 11)\n\\begintext\n";
        let err = parse_text_kernel_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Parse { .. }));
    }

    #[test]
    fn rejects_delta_at_wrong_type() {
        let kernel = "\\begindata\nDELTET/DELTA_T_A=32.184\nDELTET/K=1.657D-3\nDELTET/EB=1.671D-2\nDELTET/M=(6.239996D0 1.99096871D-7)\nDELTET/DELTA_AT=(10 1972)\n\\begintext\n";
        let err = parse_text_kernel_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Parse { .. }));
    }

    #[test]
    fn rejects_deltet_m_wrong_length() {
        let kernel = "\\begindata\nDELTET/DELTA_T_A=32.184\nDELTET/K=1.657D-3\nDELTET/EB=1.671D-2\nDELTET/M=(6.239996D0)\nDELTET/DELTA_AT=(10 @1972-JAN-1)\n\\begintext\n";
        let err = parse_text_kernel_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Parse { .. }));
    }

    #[test]
    fn rejects_incomplete_deltet_block() {
        let kernel = "\\begindata\nDELTET/DELTA_T_A = 32.184\n\\begintext\n";
        let err = parse_text_kernel_from_str(kernel).unwrap_err();
        assert!(matches!(err, TextKernelError::Parse { .. }));
    }
}
