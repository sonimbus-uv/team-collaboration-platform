use super::content::EmailContent;

pub fn verification_email_content(locale: &str, code: &str) -> EmailContent {
    let (subject, message) = if matches!(locale, "en" | "en-US") {
        ("Your Sonimbus verification code", "Enter this code in Sonimbus to verify your email address. It expires 10 minutes after the request. Resending does not extend the deadline. Do not share it. If you did not request this, ignore this message.")
    } else {
        ("Tu código de verificación de Sonimbus", "Introduce este código en Sonimbus para verificar tu correo electrónico. Vence a los 10 minutos de solicitarlo. Reenviarlo no amplía el plazo. No lo compartas. Si no lo solicitaste, ignora este mensaje.")
    };
    let safe_code = super::html::escape_html(code);

    EmailContent {
        subject,
        text: format!("{message}\n\n{code}"),
        html: format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body><h1>{subject}</h1><p>{message}</p><p style=\"font-size:28px;letter-spacing:4px\">{safe_code}</p></body></html>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn code_mail_has_no_activation_link_or_external_resources() {
        for locale in ["es-MX", "en"] {
            let mail = verification_email_content(locale, "<test-code>");
            assert!(!mail.html.contains("href="));
            assert!(!mail.html.contains("src="));
            assert!(!mail.html.contains("<test-code>"));
            assert!(mail.html.contains("&lt;test-code&gt;"));
            assert!(mail.text.contains("10"));
        }
    }
}
