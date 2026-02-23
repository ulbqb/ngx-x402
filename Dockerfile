# Build nginx from source + x402 module (ensures binary compatibility)
FROM debian:bookworm-slim AS builder
ARG NGINX_VERSION=1.28.2

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential clang libclang-dev libc6-dev zlib1g-dev libpcre2-dev pkg-config libssl-dev \
    ca-certificates curl wget \
    && rm -rf /var/lib/apt/lists/*

# nginx: --with-compat + -fPIC required for dynamic modules
RUN wget -q https://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz \
    && tar xzf nginx-${NGINX_VERSION}.tar.gz \
    && cd nginx-${NGINX_VERSION} \
    && ./configure \
        --prefix=/etc/nginx \
        --sbin-path=/usr/sbin/nginx \
        --modules-path=/usr/lib/nginx/modules \
        --conf-path=/etc/nginx/nginx.conf \
        --error-log-path=/var/log/nginx/error.log \
        --http-log-path=/var/log/nginx/access.log \
        --pid-path=/run/nginx.pid \
        --with-compat \
        --with-http_ssl_module \
        --with-stream \
        --with-cc-opt="-g -O2 -fPIC" \
        --with-ld-opt="-Wl,--as-needed -pie" \
    && make -j$(nproc) \
    && mv /nginx-${NGINX_VERSION} /nginx

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /build
COPY . .

ENV NGINX_SOURCE_DIR=/nginx
ENV NGINX_BINARY_PATH=/nginx/objs/nginx
RUN cargo build --release --features export-modules

# Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpcre2-8-0 zlib1g libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false nginx \
    && mkdir -p /usr/lib/nginx/modules /etc/nginx/conf.d /var/cache/nginx /var/log/nginx /run \
    && chown -R nginx:nginx /var/cache/nginx /var/log/nginx

COPY --from=builder /nginx/objs/nginx /usr/sbin/nginx
COPY --from=builder /build/target/release/libngx_x402.so /usr/lib/nginx/modules/
COPY nginx.conf.example /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
