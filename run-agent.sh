#!/bin/bash
# CodingAgent 快速启动脚本

set -e

# 确保在正确的目录
cd "$(dirname "$0")"

# 加载环境变量
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# 加载 Rust 环境
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# 检查是否需要编译
if [ ! -f "coding-agent/target/debug/coding-agent" ]; then
    echo "📦 第一次使用，正在编译..."
    cd coding-agent
    cargo build
    cd ..
fi

# 运行程序
cd coding-agent
exec ./target/debug/coding-agent
