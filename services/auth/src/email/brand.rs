pub struct EmailBrand {
    pub app_name: &'static str,
    pub logo_url: &'static str,
    pub primary_color: &'static str,
    pub background_color: &'static str,
    pub text_color: &'static str,
}

pub const SONIMBUS_BRAND: EmailBrand = EmailBrand {
    app_name: "Sonimbus",
    logo_url: "https://sonimbus.com/assets/logo.png",
    primary_color: "#0e8318",
    background_color: "#F8FAFC",
    text_color: "#0F172A",
};
