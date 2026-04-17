use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigResponse {
    pub referral_code: String,
    pub daily_limit_fmt: String,
    pub monthly_limit_fmt: String,
    pub app_version: String,
    pub build_number: String,
    pub company_name: String,
    pub copyright_text: String,
    pub support_email: String,
    pub certificate_url: String, // URL to the PDF
}
