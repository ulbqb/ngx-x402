# Build nginx 1.28.2 from source + Rust module (ensures module/nginx binary compatibility)
FROM debian:bookworm-slim AS builder

ENV NGINX_VERSION=1.28.2

RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    libc6-dev \
    zlib1g-dev \
    libpcre2-dev \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Download and build nginx (--with-compat for dynamic modules, -fPIC for .so)
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
        --with-file-aio \
        --with-threads \
        --with-http_addition_module \
        --with-http_auth_request_module \
        --with-http_dav_module \
        --with-http_flv_module \
        --with-http_gunzip_module \
        --with-http_gzip_static_module \
        --with-http_mp4_module \
        --with-http_random_index_module \
        --with-http_realip_module \
        --with-http_secure_link_module \
        --with-http_slice_module \
        --with-http_ssl_module \
        --with-http_stub_status_module \
        --with-http_sub_module \
        --with-http_v2_module \
        --with-http_v3_module \
        --with-mail \
        --with-mail_ssl_module \
        --with-stream \
        --with-stream_realip_module \
        --with-stream_ssl_module \
        --with-stream_ssl_preread_module \
        --with-debug \
        --with-cc-opt="-g -O2 -fPIC" \
        --with-ld-opt="-Wl,--as-needed -pie" \
    && make -j$(nproc)

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build
COPY . .

# Build module against our nginx source (ensures signature compatibility)
ENV NGINX_SOURCE_DIR=/nginx-1.28.2
ENV NGINX_BINARY_PATH=/nginx-1.28.2/objs/nginx
# Build x402 module (test_stubs excluded unless integration-test feature)
RUN NGINX_SOURCE_DIR=/nginx-1.28.2 NGINX_BINARY_PATH=/nginx-1.28.2/objs/nginx \
    cargo build --release --features export-modules

# Runtime: use our built nginx (guaranteed binary compatibility with module)
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpcre2-8-0 zlib1g libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false nginx

COPY --from=builder /nginx-1.28.2/objs/nginx /usr/sbin/nginx
RUN mkdir -p /usr/lib/nginx/modules /etc/nginx/logs /etc/nginx/conf.d \
    /var/cache/nginx/client_temp /var/cache/nginx/proxy_temp /var/log/nginx /run \
    && chown -R nginx:nginx /var/cache/nginx /var/log/nginx /etc/nginx/logs

COPY --from=builder /build/target/release/libngx_x402.so /usr/lib/nginx/modules/
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
