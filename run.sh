#!/bin/bash

# CodingAgent 启动脚本
# 自动加载环境变量并运行项目

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查 .env 文件是否存在
if [ ! -f .env ]; then
    echo -e "${RED}错误: .env 文件不存在${NC}"
    echo -e "${YELLOW}请先复制 .env.example 到 .env 并配置你的 API 密钥:${NC}"
    echo "  cp .env.example .env"
    echo "  然后编辑 .env 文件，设置 ANTHROPIC_API_KEY"
    exit 1
fi

# 加载环境变量
set -a
source .env
set +a

# 检查 API 密钥是否已配置
if [ "$ANTHROPIC_API_KEY" = "your-api-key-here" ]; then
    echo -e "${RED}错误: ANTHROPIC_API_KEY 未配置${NC}"
    echo -e "${YELLOW}请在 .env 文件中设置你的 Anthropic API 密钥${NC}"
    echo "获取 API Key: https://console.anthropic.com/settings/keys"
    exit 1
fi

# 显示配置信息
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  CodingAgent 启动中...${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "📦 模型: ${YELLOW}$AGENT_MODEL${NC}"
echo -e "📁 会话目录: ${YELLOW}$SESSION_DIR${NC}"
echo -e "🔑 API Key: ${YELLOW}${ANTHROPIC_API_KEY:0:10}...${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""

# 确保在正确的目录
cd "$(dirname "$0")"

# 加载 Rust 环境（如果使用 rustup）
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# 运行项目
echo -e "${GREEN}正在运行...${NC}"
echo ""

cargo run -- "$@"
