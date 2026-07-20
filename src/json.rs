//! Minimal JSON serialization helpers shared by the HTML renderers.
//!
//! The generated documents embed their data as a JSON object spliced into a
//! `<script>` element, so string escaping additionally guards `<` — the
//! payload must never be able to terminate the surrounding script element.

/// Appends `value` as a JSON string literal. `<` is escaped so the payload can
/// never terminate the surrounding `<script>` element.
pub(crate) fn push_str(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Appends `value` as a JSON string literal, or `null`.
pub(crate) fn push_opt_str(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => push_str(out, value),
        None => out.push_str("null"),
    }
}

/// Appends `value` as a JSON number, or `null`.
pub(crate) fn push_opt_num(out: &mut String, value: Option<f64>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped_and_script_safe() {
        let mut out = String::new();
        push_str(&mut out, "a\"b\\c\n</script>");
        assert_eq!(out, "\"a\\\"b\\\\c\\n\\u003c/script>\"");
    }

    #[test]
    fn optional_values_render_null() {
        let mut out = String::new();
        push_opt_str(&mut out, None);
        out.push(',');
        push_opt_num(&mut out, None);
        out.push(',');
        push_opt_num(&mut out, Some(92.5));
        assert_eq!(out, "null,null,92.5");
    }
}
