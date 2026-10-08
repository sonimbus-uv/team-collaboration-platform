use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub fn verification_success_page() -> String {
    page(
        "Correo verificado",
        "Tu correo quedó verificado. Vuelve a Sonimbus para continuar.",
        "success",
        "✓",
        "Todo listo",
        "Ya puedes cerrar esta pestaña y continuar en la aplicación.",
    )
}

pub fn verification_invalid_page() -> String {
    page(
        "Enlace no disponible",
        "Este enlace no es válido o ya no puede utilizarse. Solicita uno nuevo desde Sonimbus.",
        "invalid",
        "!",
        "Revisa tu enlace",
        "Si ya verificaste tu correo, puedes continuar en la aplicación.",
    )
}

pub fn verification_unavailable_page() -> String {
    page(
        "No pudimos completar la verificación",
        "Inténtalo de nuevo más tarde.",
        "unavailable",
        "…",
        "Inténtalo más tarde",
        "Vuelve a abrir el enlace del correo cuando el servicio esté disponible.",
    )
}

fn page(title: &str, message: &str, state: &str, icon: &str, label: &str, hint: &str) -> String {
    // Only trusted static messages are interpolated. Never render tokens or account data.
    let styles = r#"
:root { color-scheme: light; font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #0f172a; background: #f8fafc; }
* { box-sizing: border-box; }
body { margin: 0; min-height: 100vh; min-height: 100svh; display: grid; place-items: center; padding: 32px 20px; background: radial-gradient(ellipse at top, #e7f5e9, transparent 65%), #f8fafc; }
main { width: 100%; max-width: 480px; }
.brand { text-align: center; font-weight: 750; font-size: 22px; letter-spacing: -.5px; margin: 0 0 24px; }
.brand span { color: #0e8318; }
.card { background: #fff; border: 1px solid #e2e8f0; border-radius: 24px; padding: 40px 32px; text-align: center; box-shadow: 0 20px 60px #0f172a0a; }
.icon { display: grid; place-items: center; width: 72px; height: 72px; border-radius: 50%; margin: 0 auto 24px; font-size: 36px; font-weight: 600; background: #eaf7ed; color: #0e8318; }
.invalid .icon { background: #fff4df; color: #925700; }
.unavailable .icon { background: #edf2ff; color: #4053a4; }
.label { font-size: 12px; font-weight: 700; letter-spacing: 1.5px; text-transform: uppercase; color: #64748b; margin: 0 0 12px; }
h1 { font-size: clamp(24px, 5vw, 30px); line-height: 1.2; letter-spacing: -.7px; margin: 0 0 16px; }
.message { font-size: 16px; line-height: 1.7; color: #475569; margin: 0; }
.hint { border-top: 1px solid #e2e8f0; padding-top: 24px; margin: 28px 0 0; font-size: 14px; line-height: 1.6; color: #64748b; }
footer { text-align: center; margin-top: 24px; font-size: 12px; color: #64748b; }
@media (max-width: 400px) { .card { padding: 32px 24px; } }
"#;
    format!(
        r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} | Sonimbus</title><style>{styles}</style></head>
<body><main>
<p class="brand"><span>●</span> Sonimbus</p>
<section class="card {state}" aria-labelledby="page-title">
<div class="icon" aria-hidden="true">{icon}</div>
<p class="label">{label}</p>
<h1 id="page-title">{title}</h1>
<p class="message">{message}</p>
<p class="hint">{hint}</p>
</section>
<footer>Sonimbus · Verificación de correo</footer>
</main></body></html>"#
    )
}

pub fn verification_page_response(status: StatusCode, page: String) -> Response {
    (
        status,
        [
            ("cache-control", "no-store"),
            ("referrer-policy", "no-referrer"),
            (
                "content-security-policy",
                "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            ),
            ("x-content-type-options", "nosniff"),
        ],
        Html(page),
    )
        .into_response()
}
