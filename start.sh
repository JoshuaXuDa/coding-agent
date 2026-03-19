#!/bin/bash
# CodingAgent 启动脚本

# 加载环境变量
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# 加载 Rust 环境
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# 运行程序
cd coding-agent && cargo run
