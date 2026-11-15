use super::content::EmailContent;

pub fn google_link_notice(locale: &str) -> EmailContent {
    let (subject, message) = if matches!(locale, "en" | "en-US") {
        ("Google was connected to your Sonimbus account", "A Google account was connected to your Sonimbus account. If this was not you, review your account security and contact support. This notice does not contain a verification code and does not require a reply.")
    } else {
        ("Se conectó Google a tu cuenta de Sonimbus", "Se conectó una cuenta de Google a tu cuenta de Sonimbus. Si no fuiste tú, revisa la seguridad de tu cuenta y contacta con soporte. Este aviso no contiene un código de verificación ni requiere una respuesta.")
    };

    EmailContent {
        subject,
        text: message.into(),
        html: format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body><h1>{subject}</h1><p>{message}</p></body></html>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notices_have_no_credentials_or_external_resources() {
        for locale in ["es-MX", "en"] {
            let notice = google_link_notice(locale);
            assert!(!notice.html.contains("href="));
            assert!(!notice.html.contains("src="));
            assert!(notice.text.contains("Google"));
        }
    }
}
