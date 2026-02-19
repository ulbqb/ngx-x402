FROM rust:1.82-bookworm AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    libc6-dev \
    zlib1g-dev \
    pkg-config \
    libssl-dev \
    nginx \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

ENV NGX_CONFIGURE_ARGS="--without-http_rewrite_module"
RUN cargo build --release --features export-modules

FROM nginx:1.28

COPY --from=builder /build/target/release/libngx_x402.so /usr/lib/nginx/modules/libngx_x402.so
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
