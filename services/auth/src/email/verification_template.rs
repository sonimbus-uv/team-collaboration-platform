pub struct VerificationEmailContent {
    pub subject: &'static str,
    pub text: String,
    pub html: String,
}

pub fn verification_email_content(locale: &str, code: &str) -> VerificationEmailContent {
    let english = matches!(locale, "en" | "en-US");
    let (subject, message) = if english {
        ("Complete your Sonimbus registration", "Enter this code in the application to complete your Sonimbus registration. It expires 10 minutes after the registration request; resending does not extend that deadline. Do not share it. If you did not request this registration, ignore this message.")
    } else {
        ("Completa tu registro en Sonimbus", "Introduce este código en la aplicación para completar tu registro en Sonimbus. Vence a los 10 minutos de solicitar el registro; reenviarlo no amplía ese plazo. No lo compartas. Si no solicitaste el registro, ignora este mensaje.")
    };
    let safe_code = code
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    VerificationEmailContent {
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
