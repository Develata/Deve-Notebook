# 多阶段构建 Dockerfile
# 目标：768MB VPS 上的精简运行时镜像（<200MB）
# 包含后端 API（Axum）和前端 SPA（Leptos）

# 阶段 1: chef — 构建依赖解析环境
FROM rust:1.85-bookworm AS chef
RUN cargo install cargo-chef && \
    cargo install trunk && \
    rustup target add wasm32-unknown-unknown && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs
WORKDIR /app

# 阶段 2: planner — 准备 Cargo 依赖锁定
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# 阶段 3: deps — 编译后端依赖（缓存层）
FROM chef AS deps
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package deve_cli --package deve_core

# 阶段 4: frontend — 编译前端资源
FROM chef AS frontend
COPY . .
WORKDIR /app/apps/web
# npm 构建：编译 editor.bundle.js，复制 KaTeX 到 public/
RUN npm ci --ignore-scripts && npm run build
# Trunk 构建：生成 Leptos WASM（输出到 apps/web/dist/）
RUN trunk build --release

# 阶段 5: backend — 编译后端二进制
FROM deps AS backend
COPY . .
# 将前端 dist 放入 CLI build script 的默认扫描路径，编译进单二进制。
COPY --from=frontend /app/apps/web/dist/ /app/apps/web/dist/
RUN cargo build --release --package deve_cli && \
    strip target/release/deve_cli

# 阶段 6: runtime — 精简运行时镜像
FROM debian:bookworm-slim AS runtime
# 安装 ca-certificates 和 curl（健康检查用，纯 Rust 加密库无需 openssl）
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -u 1000 -s /bin/bash appuser && \
    mkdir -p /data && \
    chown -R appuser:appuser /data

# 复制后端二进制
COPY --from=backend /app/target/release/deve_cli /usr/local/bin/deve_cli
RUN chmod +x /usr/local/bin/deve_cli

# 环境变量配置
ENV DEVE_VAULT_PATH=/data/vault
ENV DEVE_BIND_ADDR=0.0.0.0:3001

EXPOSE 3001
VOLUME /data

# 切换至非 root 用户
USER appuser

# 健康检查
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -sf http://localhost:3001/api/node/role || exit 1

# 启动命令
CMD ["deve_cli", "serve", "--port", "3001"]
