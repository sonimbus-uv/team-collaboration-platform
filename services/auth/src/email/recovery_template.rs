use super::content::EmailContent;

pub fn recovery_email_content(locale: &str, code: &str) -> EmailContent {
    let (subject, before, after) = if matches!(locale, "en" | "en-US") {
        ("Reset your Sonimbus password",
         "Hello,\n\nWe received a request to reset your Sonimbus account password.\n\nYour recovery code is:",
         concat!(
            "Enter this code in the application where you requested recovery and choose a new password.\n\n",
            "The code expires in 10 minutes and can only be used once.\n\n",
            "Do not share this code. The Sonimbus team will never ask for it by email, phone, or chat.\n\n",
            "If you did not request this change, ignore this message. This request alone does not change your password or sign you out.\n\n",
            "After you complete the change, your sessions will be signed out and you will need to sign in with your new password.\n\n",
            "The Sonimbus team",
        ))
    } else {
        ("Recupera tu contraseña de Sonimbus",
         "Hola:\n\nRecibimos una solicitud para restablecer la contraseña de tu cuenta de Sonimbus.\n\nTu código de recuperación es:",
         concat!(
            "Introduce este código en la aplicación donde solicitaste la recuperación y elige una nueva contraseña.\n\n",
            "El código vence en 10 minutos y solo puede usarse una vez.\n\n",
            "No compartas este código. El equipo de Sonimbus nunca te lo pedirá por correo, teléfono o chat.\n\n",
            "Si no solicitaste este cambio, ignora este mensaje. Esta solicitud por sí sola no cambia tu contraseña ni cierra tus sesiones.\n\n",
            "Cuando completes el cambio, se cerrarán tus sesiones y tendrás que iniciar sesión con la nueva contraseña.\n\n",
            "Equipo Sonimbus",
        ))
    };
    
    EmailContent {
        subject,
        text: format!("{before}\n\n{code}\n\n{after}"),
        html: format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body><h1>{subject}</h1><p>{}</p><p style=\"font-size:28px;letter-spacing:4px\">{}</p><p>{}</p></body></html>", super::html::escape_html(before).replace('\n', "<br>"), super::html::escape_html(code), super::html::escape_html(after).replace('\n', "<br>")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_mail_escapes_codes_without_activation_links_or_external_resources() {
        for locale in ["es-MX", "en", "en-US"] {
            let content = recovery_email_content(locale, "<test-code>&");
            assert!(content.html.contains("&lt;test-code&gt;&amp;"));
            assert!(!content.html.contains("<test-code>"));
            assert!(content.text.contains("<test-code>&"));
            assert!(!content.html.contains("href="));
            assert!(!content.html.contains("src="));
        }
    }
}
