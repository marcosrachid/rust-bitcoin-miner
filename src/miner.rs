use std::{
    io::{self, Write},
    str::FromStr,
    time::Instant,
};

use anyhow::{anyhow, bail, Context, Result};
use bitcoin::blockdata::block::{Block, Header};
use bitcoin::blockdata::script::Builder as ScriptBuilder;
use bitcoin::blockdata::transaction::{OutPoint, Transaction, TxIn, TxOut};
use bitcoin::consensus::encode::serialize;
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::{absolute, Amount, Address, Network, ScriptBuf, TxMerkleNode, Txid};
use bitcoincore_rpc::{Client, RpcApi};

use crate::gbt::GbtTemplate;

pub struct MinerStats {
    total_hashes: u64,
    last_report_hashes: u64,
    start: Instant,
    last_report: Instant,
    report_every: u32,
}

/// Duas linhas fixas no topo: (1) hashrate, (2) status — sempre reescritas no lugar (ANSI).
struct TerminalUi {
    hashrate_line: String,
    status_line: String,
}

impl TerminalUi {
    fn new() -> Self {
        Self {
            hashrate_line: "hashrate: --".to_string(),
            status_line: "status: iniciando…".to_string(),
        }
    }

    fn init(&mut self) {
        println!("{}", self.hashrate_line);
        println!("{}", self.status_line);
        // Cursor fica na linha 3; `redraw` sobe 2 linhas e reescreve as duas.
    }

    /// Reescreve linha 1 (hashrate) e linha 2 (status) sem criar linhas novas.
    fn redraw(&self) {
        let mut out = io::stdout().lock();
        let _ = write!(
            out,
            "\x1b[2A\r\x1b[2K{}\n\r\x1b[2K{}\n",
            self.hashrate_line, self.status_line
        );
        let _ = out.flush();
    }

    fn set_hashrate(&mut self, text: impl Into<String>) {
        self.hashrate_line = text.into();
        self.redraw();
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status_line = text.into();
        self.redraw();
    }
}

impl MinerStats {
    pub fn new(report_every: u32) -> Self {
        Self {
            total_hashes: 0,
            last_report_hashes: 0,
            start: Instant::now(),
            last_report: Instant::now(),
            report_every,
        }
    }

    fn tick_one_hash(&mut self, ui: &mut TerminalUi) {
        self.total_hashes += 1;
        if self.report_every == 0 {
            return;
        }
        if self.total_hashes % self.report_every as u64 != 0 {
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_report).as_secs_f64();
        if dt <= 0.0 {
            return;
        }

        let dh = (self.total_hashes - self.last_report_hashes) as f64;
        let hps = dh / dt;
        let total_hps = (self.total_hashes as f64) / self.start.elapsed().as_secs_f64().max(1e-9);
        ui.set_hashrate(format!(
            "hashrate: {:.2} MH/s (inst) | {:.2} MH/s (médio) | hashes={}",
            hps / 1e6,
            total_hps / 1e6,
            self.total_hashes
        ));

        self.last_report = now;
        self.last_report_hashes = self.total_hashes;
    }
}

pub fn parse_network(s: &str) -> Result<Network> {
    match s {
        "regtest" => Ok(Network::Regtest),
        "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Bitcoin),
        other => bail!("network inválida: {other} (use regtest|testnet|mainnet)"),
    }
}

fn bits_to_target(bits_hex: &str) -> Result<[u8; 32]> {
    // bits é "compact" (nBits). Convertemos para target 256-bit (big-endian bytes).
    let bits = u32::from_str_radix(bits_hex, 16)
        .with_context(|| format!("bits inválido: {bits_hex}"))?;

    let exponent = ((bits >> 24) & 0xff) as u32;
    let mantissa = bits & 0x00ff_ffff;

    if mantissa == 0 {
        bail!("bits mantissa = 0");
    }
    if exponent < 3 || exponent > 32 {
        bail!("bits exponent fora do esperado: {exponent}");
    }

    // target = mantissa * 256^(exponent-3)
    // Em big-endian de 32 bytes, isso equivale a posicionar a mantissa (3 bytes)
    // começando em `start = 32 - exponent`, e preencher o restante com zeros.
    let m = [(mantissa >> 16) as u8, (mantissa >> 8) as u8, mantissa as u8];
    let mut target2 = [0u8; 32];
    let start = 32usize.saturating_sub(exponent as usize);
    if start + 3 > 32 {
        bail!("bits gera target fora do range");
    }
    target2[start] = m[0];
    target2[start + 1] = m[1];
    target2[start + 2] = m[2];
    Ok(target2)
}

fn hex32_to_arr_be(s: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(s).with_context(|| format!("hex inválido (32 bytes): {s}"))?;
    if bytes.len() != 32 {
        bail!("esperado 32 bytes, veio {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn hash_meets_target(header_ser: &[u8], target_be: &[u8; 32]) -> bool {
    let digest = sha256d::Hash::hash(header_ser);
    digest.to_byte_array() <= *target_be
}

fn coinbase_tx(height: u32, value: u64, script_pubkey: ScriptBuf, extranonce: u32) -> Transaction {
    // scriptSig com BIP34: push height + tag + extranonce
    let script_sig = ScriptBuilder::new()
        .push_int(height as i64)
        .push_slice(b"rust-bitcoin-miner")
        .push_slice(&extranonce.to_le_bytes())
        .into_script();

    Transaction {
        version: bitcoin::transaction::Version::ONE,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig,
            sequence: bitcoin::Sequence::MAX,
            witness: bitcoin::Witness::default(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(value),
            script_pubkey,
        }],
    }
}

pub fn payout_address(addr: &str, network: Network) -> Result<Address> {
    Address::from_str(addr)
        .with_context(|| "endereço inválido")?
        .require_network(network)
        .map_err(|_| anyhow!("endereço não bate com a network {network:?}"))
}

pub fn mine_forever(
    client: &Client,
    payout: &Address,
    max_tries: u32,
    report_every: u32,
) -> Result<()> {
    let mut ui = TerminalUi::new();
    ui.init();

    let mut stats = MinerStats::new(report_every);

    loop {
        // 1) Pedir um "trabalho" ao nó via getblocktemplate (GBT).
        // O template contém os parâmetros do próximo bloco: prevhash, bits/target, altura, coinbasevalue, etc.
        // Alguns nós exigem declarar suporte à regra "segwit" explicitamente.
        let tmpl: GbtTemplate = client
            .call(
                "getblocktemplate",
                &[serde_json::json!({ "rules": ["segwit"] })],
            )
            .with_context(|| "getblocktemplate falhou")?;

        let prev_short = tmpl
            .previousblockhash
            .chars()
            .take(16)
            .collect::<String>();
        ui.set_status(format!(
            "status: minerando | altura {} | prev {}…",
            tmpl.height, prev_short
        ));

        // 2) Parse do hash do bloco anterior (prev_blockhash), que "ancora" o próximo bloco na cadeia.
        let prev_hash =
            bitcoin::BlockHash::from_str(&tmpl.previousblockhash).with_context(|| "prev hash")?;

        // 3) Determinar o target (limiar de dificuldade) como número de 256 bits.
        // Alguns GBTs trazem "target" já explícito; caso contrário, convertemos a forma compacta (bits/nBits).
        let target_be = if let Some(t) = tmpl.target.as_deref() {
            hex32_to_arr_be(t)?
        } else {
            bits_to_target(&tmpl.bits)?
        };

        // 4) ScriptPubKey do pagamento da coinbase.
        // Como este miner monta um bloco "vazio" (só coinbase), todo o valor vai para esse endereço.
        let script_pubkey = payout.script_pubkey();
        let mut extranonce: u32 = 0;

        // 5) Tentamos alguns ciclos de "extranonce" para variar a coinbase e, portanto, o merkle root.
        // Isso permite tentar mais hashes mesmo se o nonce de 32 bits "rodar" sem sucesso.
        for _ in 0..10u32 {
            // 5.1) Criar coinbase (única tx obrigatória). Ela inclui height (BIP34) e um extranonce.
            // Aqui não incluímos witness commitment (segwit) nem transações do mempool — é intencional
            // para manter o exemplo simples e funcional em regtest.
            let cb = coinbase_tx(tmpl.height, tmpl.coinbasevalue, script_pubkey.clone(), extranonce);
            extranonce = extranonce.wrapping_add(1);

            // 5.2) Calcular o TXID da coinbase (sem witness). Em bloco com só coinbase,
            // o merkle root é derivado diretamente do txid (por convenção de merkle tree).
            let cb_txid: Txid = cb.compute_txid();
            let merkle_root = TxMerkleNode::from_raw_hash(*cb_txid.as_raw_hash());

            // 5.3) Montar o header do bloco com campos do template.
            // O que muda no loop interno é apenas o `nonce` (e no loop externo, o merkle_root via extranonce).
            let mut header = Header {
                version: bitcoin::block::Version::from_consensus(tmpl.version),
                prev_blockhash: prev_hash,
                merkle_root,
                time: tmpl.curtime,
                bits: bitcoin::CompactTarget::from_consensus(
                    u32::from_str_radix(&tmpl.bits, 16)
                        .with_context(|| format!("bits inválido: {}", tmpl.bits))?,
                ),
                nonce: 0,
            };

            // 6) Loop de mineração: iterar nonces e testar se o hash do header (sha256d) está <= target.
            // `max_tries` limita o esforço antes de pedir um template novo (para não minerar em cima de trabalho velho).
            let mut tried: u32 = 0;
            while tried < max_tries {
                // 6.1) Serializar o header exatamente como na rede e computar o sha256d dele.
                let header_ser = serialize(&header);
                stats.tick_one_hash(&mut ui);

                // 6.2) Se o digest atende o target, temos um bloco válido (PoW).
                if hash_meets_target(&header_ser, &target_be) {
                    // 6.3) Montar o bloco completo (header + lista de transações). Aqui: só coinbase.
                    let block = Block {
                        header,
                        txdata: vec![cb],
                    };
                    let block_hex = hex::encode(serialize(&block));

                    // 6.4) Submeter o bloco ao nó. Se aceito, ele passa a propagar/validar na rede dele.
                    let submit: serde_json::Value = client
                        .call("submitblock", &[serde_json::json!(block_hex)])
                        .with_context(|| "submitblock falhou")?;

                    // 6.5) `submitblock` retorna null quando o bloco foi aceito (ou já conhecido).
                    if submit.is_null() {
                        ui.set_status(format!(
                            "status: bloco aceito | {}",
                            block.block_hash()
                        ));
                    } else {
                        ui.set_status(format!("status: bloco rejeitado | {}", submit));
                    }
                    break;
                }

                // 6.6) Tentativa falhou: incrementar nonce e tentar de novo.
                header.nonce = header.nonce.wrapping_add(1);
                tried += 1;
            }
        }
    }
}

