#!/bin/bash

# 快速启动脚本 - 直接在前台运行
set -e

# 加载环境变量
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# 检查 API Key
if [ "$ANTHROPIC_API_KEY" = "your-api-key-here" ]; then
    echo "❌ 请先配置 .env 文件中的 ANTHROPIC_API_KEY"
    exit 1
fi

# 加载 Rust 环境
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# 直接运行（前台交互模式）
cd "$(dirname "$0")"
cargo run --bin coding-agent
