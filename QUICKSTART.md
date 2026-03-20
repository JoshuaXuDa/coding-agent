# 🚀 CodingAgent 快速开始

## 第一步：配置 API 密钥

```bash
# 1. 复制配置模板
cp .env.example .env

# 2. 编辑 .env 文件，修改这一行：
# ANTHROPIC_API_KEY=sk-ant-your-actual-api-key-here
# 改为你的实际 API Key
```

**获取 API Key:** https://console.anthropic.com/settings/keys

## 第二步：运行项目

```bash
# 加载环境变量
source .env

# 运行项目
cargo run
```

## 📝 使用示例

启动后，你可以输入命令：

```
You> 列出当前目录的文件
You> 读取 README.md 文件
You> 在 src/main.rs 中搜索 "println"
```

## 🛠️ 可用工具

- **bash** - 执行 shell 命令
- **read** - 读取文件内容
- **write** - 写入文件
- **glob** - 查找文件（支持模式匹配）
- **grep** - 在文件中搜索内容
- **edit** - 替换文件中的文本

## 🔧 配置选项

在 `.env` 文件中可以配置：

```bash
# 选择模型
AGENT_MODEL=claude-sonnet-4-6        # 推荐
# AGENT_MODEL=claude-opus-4-6        # 最强
# AGENT_MODEL=claude-haiku-4-5-20251001  # 最快

# 会话配置
MAX_ROUNDS=50                        # 最大对话轮数
SESSION_DIR=./sessions               # 会话存储目录
```

## ❓ 遇到问题？

- **API 密钥错误** → 检查 `.env` 文件中的 `ANTHROPIC_API_KEY`
- **编译失败** → 确保使用新的 Rust：`source "$HOME/.cargo/env"`
- **网络问题** → 已配置国内镜像，应该很快

## 📚 更多信息

详细配置请查看：[SETUP.md](SETUP.md)
