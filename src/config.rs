use std::env;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_pass: String,
    pub payout_address: String,
    pub network: String,
    pub max_tries: u32,
    pub report_every: u32,
}

fn env_var(key: &str) -> Option<String> {
    env::var(key).ok().filter(|s| !s.trim().is_empty())
}

fn env_required(key: &str) -> Result<String> {
    env_var(key).ok_or_else(|| anyhow!("faltou definir {key} no .env (ou variável de ambiente)"))
}

fn env_u32(key: &str, default_value: u32) -> Result<u32> {
    match env_var(key) {
        None => Ok(default_value),
        Some(v) => v
            .parse::<u32>()
            .with_context(|| format!("valor inválido para {key}: {v}")),
    }
}

pub fn load_config() -> Result<Config> {
    // Carrega `.env` se existir. Se não existir, seguimos só com env do processo.
    let _ = dotenvy::dotenv();

    Ok(Config {
        rpc_url: env_var("RPC_URL").unwrap_or_else(|| "http://127.0.0.1:18443".to_string()),
        rpc_user: env_required("RPC_USER")?,
        rpc_pass: env_required("RPC_PASS")?,
        payout_address: env_required("PAYOUT_ADDRESS")?,
        network: env_var("NETWORK").unwrap_or_else(|| "regtest".to_string()),
        max_tries: env_u32("MAX_TRIES", 2_000_000)?,
        report_every: env_u32("REPORT_EVERY", 200_000)?,
    })
}

