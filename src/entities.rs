//! Product names arrive HTML-escaped (`Fruit &amp; Nut`, `Dak&#8217;s`), and notifications are plain text.

/// Decodes the named and numeric character references WordPress produces.
pub fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        match rest
            .find(';')
            .and_then(|end| Some((end, reference(&rest[1..end])?)))
        {
            Some((end, ch)) => {
                out.push(ch);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn reference(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let digits = name.strip_prefix('#')?;
            let code = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_named_and_numeric_references() {
        assert_eq!(decode("Fruit &amp; Nut"), "Fruit & Nut");
        assert_eq!(decode("Dak&#8217;s &#x41;"), "Dak\u{2019}s A");
    }

    #[test]
    fn leaves_stray_ampersands_alone() {
        assert_eq!(
            decode("Salt & pepper &unknown; &"),
            "Salt & pepper &unknown; &"
        );
    }
}
