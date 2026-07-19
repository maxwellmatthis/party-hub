use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub web_assets_path: String,
    pub sqlite_db_path: String,
    pub base_url: String,
    pub public_vapid_key_path: String,
    pub private_vapid_key_path: String,
    pub smtp_server: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    pub mail_sendtype: MailSendType,
    pub port: u16,
    pub env_mode: Option<String>,
}

#[derive(Clone, Debug)]
pub enum MailSendType {
    Auto,
    Client,
    Direct,
    Invalid(String),
}

impl AppConfig {
    pub fn from_env() -> Self {
        let web_assets_path = env::var("WEB_ASSETS_PATH")
            .expect("Environment variable WEB_ASSETS_PATH must be set.");
        let sqlite_db_path = env::var("SQLITE_DB_PATH")
            .expect("Environment variable SQLITE_DB_PATH must be set.");
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let public_vapid_key_path = env::var("PUBLIC_VAPID_KEY_PATH")
            .expect("Environment variable PUBLIC_VAPID_KEY_PATH must be set for notifications to work.");
        let private_vapid_key_path = env::var("PRIVATE_VAPID_KEY_PATH")
            .expect("Environment variable PRIVATE_VAPID_KEY_PATH must be set for notifications to work.");
        let smtp_server = env::var("SMTP_SERVER").ok();
        let smtp_username = env::var("SMTP_USERNAME").ok();
        let smtp_password = env::var("SMTP_PASSWORD").ok();
        let smtp_from = env::var("SMTP_FROM").ok();

        let mail_sendtype = match env::var("MAIL_SENDTYPE") {
            Ok(value) => match value.as_str() {
                "client" => MailSendType::Client,
                "direct" => MailSendType::Direct,
                other => MailSendType::Invalid(other.to_string()),
            },
            Err(_) => MailSendType::Auto,
        };

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .unwrap_or(8080);

        let env_mode = env::var("ENV").ok();

        Self {
            web_assets_path,
            sqlite_db_path,
            base_url,
            public_vapid_key_path,
            private_vapid_key_path,
            smtp_server,
            smtp_username,
            smtp_password,
            smtp_from,
            mail_sendtype,
            port,
            env_mode,
        }
    }

    pub fn smtp_client_configured(&self) -> bool {
        self.smtp_server.is_some()
            && self.smtp_username.is_some()
            && self.smtp_password.is_some()
            && self.smtp_from.is_some()
    }

    pub fn smtp_direct_configured(&self) -> bool {
        self.smtp_from.is_some()
    }
}