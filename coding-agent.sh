#!/bin/bash

# CodingAgent 快速启动脚本
set -e

# 加载环境变量
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# 检查 API Key
if [ -z "$OPENAI_API_KEY" ] || [ "$OPENAI_API_KEY" = "your-glm-api-key-here" ]; then
    echo "Error: Please configure OPENAI_API_KEY in .env or .env.local"
    exit 1
fi

# 加载 Rust 环境
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# 确保在项目根目录
cd "$(dirname "$0")"

# 检查是否需要编译
if [ ! -f "coding-agent/target/debug/coding-agent" ]; then
    echo "Building CodingAgent..."
    cd coding-agent
    cargo build
    cd ..
fi

# 运行程序
cd coding-agent
echo "Starting CodingAgent..."
exec ./target/debug/coding-agent
