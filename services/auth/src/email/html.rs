pub(super) fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup_quotes_and_existing_entities() {
        assert_eq!(
            escape_html("<&>\"'&lt;"),
            "&lt;&amp;&gt;&quot;&#39;&amp;lt;"
        );
        assert_eq!(escape_html("123456"), "123456");
    }
}
