use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub web_assets_path: String,
    pub sqlite_db_path: String,
    pub base_url: String,
    pub public_vapid_key: String,
    pub private_vapid_key: String,
    pub admin_token: String,
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
        let web_assets_path =
            env::var("WEB_ASSETS_PATH").expect("Environment variable WEB_ASSETS_PATH must be set.");
        let sqlite_db_path =
            env::var("SQLITE_DB_PATH").expect("Environment variable SQLITE_DB_PATH must be set.");
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let public_vapid_key = load_vapid_key("PUBLIC_VAPID_KEY", "PUBLIC_VAPID_KEY_PATH");
        let private_vapid_key = load_vapid_key("PRIVATE_VAPID_KEY", "PRIVATE_VAPID_KEY_PATH");
        let admin_token = env::var("ADMIN_TOKEN").unwrap_or(String::from(""));
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
            public_vapid_key,
            private_vapid_key,
            admin_token,
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

    pub fn page_path(&self, relative_path: &str) -> String {
        join_path(&self.web_assets_path, "pages", relative_path)
    }

    pub fn static_path(&self, relative_path: &str) -> String {
        join_path(&self.web_assets_path, "static", relative_path)
    }
}

fn load_vapid_key(key_env: &str, path_env: &str) -> String {
    let key = env::var(key_env).unwrap_or_else(|_| {
        let path = env::var(path_env).unwrap_or_else(|_| {
            panic!("Environment variable {key_env} or {path_env} must be set.")
        });
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("Failed to read VAPID key from {path}: {error}"))
    });

    key.trim().to_string()
}

fn join_path(base_path: &str, directory: &str, relative_path: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    let relative_path = relative_path.trim_start_matches('/');

    if base_path.is_empty() {
        format!("/{}/{}", directory, relative_path)
    } else {
        format!("{}/{}/{}", base_path, directory, relative_path)
    }
}
