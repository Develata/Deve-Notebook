# 多阶段构建 Dockerfile
# 目标：768MB VPS 上的精简运行时镜像（<200MB）
# 包含后端 API（Axum）和前端 SPA（Leptos）

# 阶段 1: build-env — 构建前后端所需工具
FROM rust:1.97.0-bookworm AS build-env
RUN cargo install trunk --locked --version 0.21.14 && \
    rustup target add wasm32-unknown-unknown && \
    curl -fsSL https://deb.nodesource.com/setup_24.x | bash - && \
    apt-get install -y nodejs
WORKDIR /app

# 阶段 2: frontend — 编译前端资源
FROM build-env AS frontend
COPY . .
WORKDIR /app/apps/web
# npm 构建：编译 editor.bundle.js，复制 KaTeX 到 public/
RUN npm ci --ignore-scripts && npm run build
# Trunk 构建：生成 Leptos WASM（输出到 apps/web/dist/）
RUN NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build --release

# 阶段 3: backend — 编译后端二进制
FROM build-env AS backend
COPY . .
# 将前端 dist 放入 CLI build script 的默认扫描路径，编译进单二进制。
COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/
RUN cargo build --release --locked --package deve_cli --bin deve_cli && \
    strip target/release/deve_cli

# 阶段 4: runtime — 精简运行时镜像
FROM debian:bookworm-slim AS runtime
# 安装 ca-certificates 和 curl（健康检查用，纯 Rust 加密库无需 openssl）
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -u 1000 -s /bin/bash appuser && \
    mkdir -p /data /notes && \
    chown -R appuser:appuser /data /notes

# 复制后端二进制
COPY --from=backend /app/target/release/deve_cli /usr/local/bin/deve_cli
RUN chmod +x /usr/local/bin/deve_cli && \
    ln -s /usr/local/bin/deve_cli /usr/local/bin/deve

# 环境变量配置
ENV DEVE_LEDGER_DIR=/data/ledger
ENV DEVE_BIND_ADDR=0.0.0.0:3001

EXPOSE 3001
VOLUME /data
VOLUME /notes
WORKDIR /data

# 切换至非 root 用户
USER appuser

# 健康检查
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -sf http://localhost:3001/api/node/role || exit 1

# 启动命令
CMD ["sh", "-c", "if [ ! -f /data/ledger/.host/projection-locators.toml ]; then deve_cli init --repo default --projection-base /notes --path /data; fi; deve_cli serve --port 3001"]
