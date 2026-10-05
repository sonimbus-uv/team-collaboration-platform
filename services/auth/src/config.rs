use std::env;

pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub database_max_connections: u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".to_string());

        let database_url = env::var("DATABASE_URL")?;

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()?;

        Ok(Self {
            server_addr,
            database_url,
            database_max_connections,
        })
    }
}
