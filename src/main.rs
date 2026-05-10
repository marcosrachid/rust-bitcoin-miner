use anyhow::Result;
use bitcoincore_rpc::{Auth, Client};

mod config;
mod gbt;
mod miner;

fn main() -> Result<()> {
    let cfg = config::load_config()?;
    let network = miner::parse_network(&cfg.network)?;
    let payout = miner::payout_address(&cfg.payout_address, network)?;

    let client = Client::new(
        &cfg.rpc_url,
        Auth::UserPass(cfg.rpc_user.clone(), cfg.rpc_pass.clone()),
    )?;

    miner::mine_forever(&client, &payout, cfg.max_tries, cfg.report_every)
}
