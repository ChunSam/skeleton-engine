use glyphon::{Attrs, Color, Style, Weight};

pub(super) fn parse_rich_text<'a>(
    text: &str,
    default_attrs: &Attrs<'a>,
) -> Vec<(String, Attrs<'a>)> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut color_stack: Vec<Option<Color>> = vec![None];
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut i = 0usize;

    while i < text.len() {
        let rest = &text[i..];
        let tag = if rest.starts_with("[b]") {
            Some(("b", 3))
        } else if rest.starts_with("[/b]") {
            Some(("/b", 4))
        } else if rest.starts_with("[i]") {
            Some(("i", 3))
        } else if rest.starts_with("[/i]") {
            Some(("/i", 4))
        } else if rest.starts_with("[/color]") {
            Some(("/color", 8))
        } else {
            parse_color_tag(rest).map(|(_, len)| ("color", len))
        };

        if let Some((name, len)) = tag {
            if !current.is_empty() {
                let attrs = rich_attrs(
                    default_attrs,
                    *color_stack.last().unwrap(),
                    bold_depth,
                    italic_depth,
                );
                spans.push((std::mem::take(&mut current), attrs));
            }
            match name {
                "b" => bold_depth += 1,
                "/b" => bold_depth = bold_depth.saturating_sub(1),
                "i" => italic_depth += 1,
                "/i" => italic_depth = italic_depth.saturating_sub(1),
                "color" => color_stack.push(parse_color_tag(rest).and_then(|(c, _)| c)),
                "/color" if color_stack.len() > 1 => {
                    color_stack.pop();
                }
                "/color" => {}
                _ => {}
            }
            i += len;
        } else {
            let ch = rest.chars().next().unwrap();
            current.push(ch);
            i += ch.len_utf8();
        }
    }

    if !current.is_empty() || spans.is_empty() {
        let attrs = rich_attrs(
            default_attrs,
            *color_stack.last().unwrap(),
            bold_depth,
            italic_depth,
        );
        spans.push((current, attrs));
    }
    spans
}

fn rich_attrs<'a>(
    default_attrs: &Attrs<'a>,
    color: Option<Color>,
    bold_depth: usize,
    italic_depth: usize,
) -> Attrs<'a> {
    let mut attrs = default_attrs.clone();
    if let Some(color) = color {
        attrs = attrs.color(color);
    }
    if bold_depth > 0 {
        attrs = attrs.weight(Weight::BOLD);
    }
    if italic_depth > 0 {
        attrs = attrs.style(Style::Italic);
    }
    attrs
}

fn parse_color_tag(rest: &str) -> Option<(Option<Color>, usize)> {
    let value = rest.strip_prefix("[color=")?;
    let end = value.find(']')?;
    let raw = &value[..end];
    Some((parse_color(raw), "[color=".len() + end + 1))
}

fn parse_color(raw: &str) -> Option<Color> {
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    // `len()` is BYTES, but the slices below are byte slices — so a 6- or 8-BYTE non-ASCII
    // value ("가나" is 6 bytes / 2 chars) passes the length check and then splits mid-character,
    // which panics. A hex colour is ASCII by definition, so rejecting non-ASCII here is both the
    // minimal guard and the correct one. Reached from ordinary game data: any `[color=…]` tag in
    // a dialogue or UI string, i.e. from a translator's file, not from engine code.
    if !hex.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).ok()?
    } else {
        255
    };
    Some(Color::rgba(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 6- or 8-BYTE non-ASCII colour value must be rejected, not byte-sliced.
    ///
    /// `hex.len()` counts bytes but the slices are byte slices, so "가나" (6 bytes, 2 chars)
    /// passed the length check and then split mid-character — a panic, reached from ordinary
    /// game data: any `[color=…]` tag in a dialogue or UI string, i.e. from a translator's file.
    #[test]
    fn non_ascii_color_value_is_rejected_not_sliced() {
        assert_eq!(parse_color("가나"), None, "6-byte non-ASCII must not panic");
        assert_eq!(parse_color("#가나"), None);
        assert_eq!(parse_color("가나다까"), None, "8-byte case");
        // The valid forms still parse.
        assert!(parse_color("#ff8800").is_some());
        assert!(parse_color("ff8800cc").is_some());
    }
}
