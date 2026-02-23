# x402 Payment Client

Node.js client for testing payments against ngx-x402 protected endpoints.

## Setup

```bash
cp .env.example .env
# Edit .env and set EVM_PRIVATE_KEY to your Base Sepolia wallet private key
```

## Prerequisites

- Base Sepolia testnet ETH (for gas)
- Base Sepolia USDC (from [CDP Faucet](https://portal.cdp.coinbase.com/products/faucet))
- Copy nginx.conf.example to nginx.conf and update `x402_pay_to` with your receiving address

## Run

1. Start the proxy: `docker compose up --build -d` (from project root)
2. Run the client:

```bash
npm start
# or with custom URL:
RESOURCE_SERVER_URL=http://localhost:8080 npm start
```
