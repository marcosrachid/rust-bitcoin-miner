use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GbtTemplate {
    pub version: i32,
    pub previousblockhash: String,
    pub curtime: u32,
    pub bits: String,
    pub height: u32,
    pub coinbasevalue: u64,
    // "target" normalmente vem como hex com 32 bytes
    pub target: Option<String>,
}

