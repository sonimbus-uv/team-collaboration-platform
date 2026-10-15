use super::brand::SONIMBUS_BRAND;

pub struct VerificationEmailContent {
    pub subject: &'static str,
    pub text: String,
    pub html: String,
}

pub fn verification_email_content(
    locale: &str,
    verification_url: &str,
) -> VerificationEmailContent {
    match locale {
        "en" | "en-US" => build_email(verification_url, true),
        _ => build_email(verification_url, false),
    }
}

fn build_email(verification_url: &str, english: bool) -> VerificationEmailContent {
    let brand = &SONIMBUS_BRAND;
    let app_name = escape_html(brand.app_name);
    let logo_url = escape_html(brand.logo_url);
    let primary_color = brand.primary_color;
    let background_color = brand.background_color;
    let text_color = brand.text_color;
    let (language, subject, introduction, button, footer, text) = if english {
        (
            "en",
            "Verify your email",
            format!("Thanks for registering with {app_name}. Please confirm your email address."),
            "Verify email",
            "This link expires in 24 hours. If you did not create this account, you can ignore this message.",
            format!("Verify your email by opening this link: {verification_url}\nThe link expires in 24 hours."),
        )
    } else {
        (
            "es",
            "Verifica tu correo",
            format!("Gracias por registrarte en {app_name}. Confirma tu correo electrónico."),
            "Verificar correo",
            "Este enlace vence en 24 horas. Si no creaste esta cuenta, puedes ignorar este mensaje.",
            format!("Verifica tu correo abriendo este enlace: {verification_url}\nEl enlace vence en 24 horas."),
        )
    };

    let verification_url = escape_html(verification_url);

    let html = format!(
        r#"
        <!doctype html>
        <html lang="{language}">
        <body style="margin:0; padding:0; background:{background_color}; font-family:Arial, sans-serif;">
            <table width="100%" cellpadding="0" cellspacing="0" role="presentation">
                <tr>
                    <td align="center" style="padding:32px 16px;">
                        <table width="100%" cellpadding="0" cellspacing="0" role="presentation" style="max-width:560px; background:#FFFFFF; border-radius:12px; overflow:hidden;">
                            <tr>
                                <td style="padding:32px; text-align:center;">
                                    <img src="{logo_url}" alt="{app_name}" width="120" style="margin-bottom:24px;" />
                                    <h1 style="margin:0; color:{text_color}; font-size:24px;">
                                        {subject}
                                    </h1>
                                    <p style="color:#475569; font-size:16px; line-height:1.5;">
                                        {introduction}
                                    </p>
                                    <a href="{verification_url}" style="display:inline-block; margin-top:16px; padding:12px 20px; background:{primary_color}; color:#FFFFFF; text-decoration:none; border-radius:8px; font-weight:bold;">
                                        {button}
                                    </a>
                                    <p style="color:#64748B; font-size:13px; line-height:1.5; margin-top:24px;">
                                        {footer}
                                    </p>
                                </td>
                            </tr>
                        </table>
                    </td>
                </tr>
            </table>
        </body>
        </html>
        "#
    );

    VerificationEmailContent {
        subject,
        text,
        html,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
