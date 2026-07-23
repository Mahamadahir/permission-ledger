use std::{env, net::SocketAddr};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub cookie_secure: bool,
    pub web_origin: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        let bind_addr = env::var("BACKEND_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()
            .expect("BACKEND_BIND_ADDR must be a socket address");
        let database_url = env::var("DATABASE_URL")?;
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|value| value == "true")
            .unwrap_or(false);
        let web_origin =
            env::var("WEB_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());

        Ok(Self {
            bind_addr,
            database_url,
            cookie_secure,
            web_origin,
        })
    }
}
