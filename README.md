# rust-bitcoin-miner

Miner simples em Rust que usa `bitcoind` via RPC:
- pega um template com `getblocktemplate`
- monta um bloco **vazio** (só coinbase)
- tenta encontrar um nonce válido (PoW)
- envia com `submitblock`

## Estrutura do código

- `src/main.rs`: ponto de entrada (carrega config, cria RPC client, chama o miner)
- `src/config.rs`: leitura do `.env`/env e validação
- `src/gbt.rs`: modelo do retorno de `getblocktemplate`
- `src/miner.rs`: loop de mineração + hashrate + construção de bloco

## Rodando em regtest (recomendado)

1) Suba um `bitcoind` em regtest com RPC habilitado (exemplo `~/.bitcoin/bitcoin.conf`):

```conf
regtest=1
server=1
rpcuser=usuario
rpcpassword=senha

[regtest]
rpcbind=127.0.0.1
rpcallowip=127.0.0.1
```

2) Inicie o nó:

```bash
bitcoind -regtest -daemon
```

3) Crie uma wallet e um endereço (para receber a coinbase):

```bash
bitcoin-cli -regtest createwallet miner
bitcoin-cli -regtest getnewaddress "" bech32
```

4) Rode o miner:

```bash
cat > .env <<'EOF'
RPC_URL=http://127.0.0.1:18443
RPC_USER=usuario
RPC_PASS=senha
NETWORK=regtest
PAYOUT_ADDRESS=<SEU_ENDERECO_BECH32>
MAX_TRIES=2000000
EOF

cargo run --release
```

## Rodando em testnet (passo a passo)

> Aviso: minerar **blocos** na testnet com CPU é extremamente improvável (dificuldade alta).  
> Essa seção serve para você rodar o fluxo completo com um nó conectado na testnet.

### 1) Configurar e iniciar o `bitcoind` na testnet

Crie/edite seu `~/.bitcoin/bitcoin.conf` (ou `~/.bitcoin/testnet3/bitcoin.conf`) com algo assim:

```conf
testnet=1
server=1
rpcuser=usuario
rpcpassword=senha

[test]
rpcbind=127.0.0.1
rpcallowip=127.0.0.1
```

Inicie o nó:

```bash
bitcoind -testnet -daemon
```

Veja info básica (opcional):

```bash
bitcoin-cli -testnet getblockchaininfo
bitcoin-cli -testnet getnetworkinfo
```

### 2) Criar wallet e endereço de recebimento (coinbase)

```bash
bitcoin-cli -testnet createwallet miner
bitcoin-cli -testnet getnewaddress "" bech32
```

O endereço retornado deve começar com `tb1...`.

### 3) Configurar o `.env` para testnet

No diretório do projeto:

```bash
cat > .env <<'EOF'
RPC_URL=http://127.0.0.1:18332
RPC_USER=usuario
RPC_PASS=senha
NETWORK=testnet
PAYOUT_ADDRESS=<SEU_ENDERECO_TB1>
MAX_TRIES=2000000
REPORT_EVERY=200000
EOF
```

### 4) Rodar o miner

```bash
cargo run --release
```

### 5) Dicas de troubleshooting (testnet)

- Se aparecer erro de autenticação RPC: confira `RPC_USER/RPC_PASS` e se o `bitcoind` está com `server=1`.
- Se o nó ainda não estiver sincronizado, você pode ver progresso em:

```bash
bitcoin-cli -testnet getblockchaininfo
```

- Se você quiser confirmar se seu nó aceitou um bloco (muito raro em CPU), monitore os logs do `bitcoind`:

```bash
tail -f ~/.bitcoin/testnet3/debug.log
```

## Rodando em mainnet (passo a passo)

> Aviso: minerar **blocos** na mainnet com CPU é impraticável (ASICs dominam a rede).  
> Use esta seção para conectar o miner ao seu nó e ver o fluxo; não espere encontrar blocos.

### 1) Configurar e iniciar o `bitcoind` na mainnet

Crie/edite seu `~/.bitcoin/bitcoin.conf` com algo assim:

```conf
server=1
rpcuser=usuario
rpcpassword=senha

[main]
rpcbind=127.0.0.1
rpcallowip=127.0.0.1
```

Inicie o nó:

```bash
bitcoind -daemon
```

Veja info básica (opcional):

```bash
bitcoin-cli getblockchaininfo
bitcoin-cli getnetworkinfo
```

### 2) Criar wallet e endereço de recebimento (coinbase)

```bash
bitcoin-cli createwallet miner
bitcoin-cli getnewaddress "" bech32
```

O endereço retornado deve começar com `bc1...`.

### 3) Configurar o `.env` para mainnet

No diretório do projeto:

```bash
cat > .env <<'EOF'
RPC_URL=http://127.0.0.1:8332
RPC_USER=usuario
RPC_PASS=senha
NETWORK=mainnet
PAYOUT_ADDRESS=<SEU_ENDERECO_BC1>
MAX_TRIES=2000000
REPORT_EVERY=200000
EOF
```

### 4) Rodar o miner

```bash
cargo run --release
```

### 5) Segurança

- **Nunca exponha o RPC** do seu `bitcoind` para a internet.
- Mantenha `rpcbind`/`rpcallowip` restritos ao localhost, como acima.

## Observações

- Em `testnet` funciona do mesmo jeito, mas a dificuldade e tempo de encontrar blocos pode ser impraticável.
- O bloco minerado aqui é “vazio” (não inclui transações do mempool). Isso simplifica bastante e ainda é válido.