# ngx-x402

Nginx module for the x402 HTTP micropayment protocol. Acts as a reverse proxy that adds x402 payment gates in front of existing APIs without modifying the backend.

## Features

- **Reverse proxy**: Place in front of any HTTP API via `proxy_pass`
- **x402 protocol**: Full HTTP 402 Payment Required flow with Facilitator verification
- **Multi-tenant**: Different `pay_to` addresses per location block
- **Dynamic pricing**: Override prices at runtime via Redis
- **Replay prevention**: SHA256-based signature tracking in Redis with configurable TTL
- **Prometheus metrics**: `/metrics` endpoint for observability
- **Browser support**: HTML paywall page for browser requests, JSON for API clients
- **Facilitator fallback**: Configurable error/pass behavior when facilitator is unavailable

## Quick Start

```bash
docker-compose up
```

## Configuration

```nginx
load_module /usr/lib/nginx/modules/libngx_x402.so;

http {
    server {
        location /api/weather {
            x402 on;
            x402_amount 0.001;
            x402_pay_to 0xYourWalletAddress;
            x402_facilitator_url https://x402.org/facilitator;
            x402_network base-sepolia;
            x402_description "Weather API";

            proxy_pass http://backend:3000/api/weather;
        }
    }
}
```

## Directives

| Directive | Example | Description |
|---|---|---|
| `x402` | `on`/`off` | Enable x402 payment verification |
| `x402_amount` | `0.001` | Payment amount (dollar-denominated) |
| `x402_pay_to` | `0xAbC...` | Receiving wallet address |
| `x402_facilitator_url` | `https://...` | Facilitator service URL |
| `x402_network` | `base-sepolia` | Network name or CAIP-2 ID |
| `x402_network_id` | `8453` | Chain ID (takes precedence over network) |
| `x402_asset` | `0x...` | Custom token address (defaults to USDC) |
| `x402_asset_decimals` | `18` | Token decimals (default: 6 for USDC) |
| `x402_description` | `"Weather API"` | Endpoint description |
| `x402_resource` | `/api/weather` | Resource path (auto-detected if omitted) |
| `x402_timeout` | `10` | Facilitator timeout in seconds |
| `x402_ttl` | `60` | Payment authorization validity in seconds |
| `x402_facilitator_fallback` | `error`/`pass` | Behavior on facilitator failure |
| `x402_redis_url` | `redis://...` | Redis URL for dynamic config |
| `x402_replay_ttl` | `86400` | Replay prevention TTL in seconds |

## Dynamic Pricing via Redis

```bash
# Override price for a path
redis-cli SET /api/weather 0.005

# Price takes effect on next request (no nginx reload needed)
```

## Building from Source

```bash
sudo apt-get install -y build-essential clang libclang-dev nginx
cargo build --release --features export-modules
```

The compiled module will be at `target/release/libngx_x402.so`.

## License

Apache-2.0
