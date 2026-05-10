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

## Porta RPC: padrão e como descobrir

O miner fala com o `bitcoind` por HTTP na **porta RPC** (não confundir com a porta **P2P**, em que o nó troca blocos com outros nós).

### Portas padrão (se você não definiu `rpcport`)

| Rede    | RPC (JSON-RPC) | P2P (rede Bitcoin) |
|---------|----------------|---------------------|
| **mainnet** | `8332`     | `8333`              |
| **testnet** | `18332`    | `18333`             |
| **regtest** | `18443`    | `18444`             |
| **signet**  | `38332`    | `38333`             |

O `RPC_URL` do `.env` deve usar a **porta RPC** (ex.: regtest → `http://127.0.0.1:18443`).

### Se você definiu `rpcport` no `bitcoin.conf`

Qualquer valor em `rpcport=` (no arquivo global ou dentro de `[main]`, `[test]`, `[regtest]`, etc.) **substitui o padrão** daquela rede. Confira no arquivo:

```bash
grep -n 'rpcport=' ~/.bitcoin/bitcoin.conf
grep -n '^\[' ~/.bitcoin/bitcoin.conf
```

(Ajuste o caminho se seu `bitcoind` usar outro diretório de dados.)

### Depois de `bitcoind -regtest -daemon` (ou outra rede): ver o que está escutando

O jeito mais direto no Linux é listar as portas TCP em que o processo `bitcoind` está em **LISTEN**:

```bash
ss -ltnp | grep bitcoind
```

Você costuma ver **duas** portas: uma é a **RPC** (ex.: `18443` no regtest), outra é a **P2P** (ex.: `18444` no regtest). Use a RPC no `RPC_URL`.

Alternativa:

```bash
sudo lsof -iTCP -sTCP:LISTEN -c bitcoind
```

### `getnetworkinfo` não mostra a porta RPC

O comando `bitcoin-cli -regtest getnetworkinfo` (e equivalentes) expõe sobretudo dados da **rede P2P**; o campo `port` aí é a porta P2P, não a RPC. Para a RPC, use `rpcport` no conf ou `ss`/`lsof` como acima.

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

## Parar o `bitcoind`

O jeito correto é pedir o encerramento limpo via RPC (o nó grava estado e fecha conexões).

| Rede    | Comando |
|---------|---------|
| **Mainnet** | `bitcoin-cli stop` |
| **Testnet** | `bitcoin-cli -testnet stop` |
| **Regtest** | `bitcoin-cli -regtest stop` |

Se você tiver **vários** `bitcoind` ao mesmo tempo (ex.: mainnet e testnet), use o flag da rede correspondente ao processo que quer parar.

Verificar se ainda há processo rodando:

```bash
pgrep -a bitcoind || echo "nenhum bitcoind"
```

Só em último caso (travou e não responde a `stop`): encerre pelo PID, por exemplo `kill <PID>` ou `kill -9 <PID>` após `pgrep bitcoind`.

## Observações

- Em `testnet` funciona do mesmo jeito, mas a dificuldade e tempo de encontrar blocos pode ser impraticável.
- O bloco minerado aqui é “vazio” (não inclui transações do mempool). Isso simplifica bastante e ainda é válido.