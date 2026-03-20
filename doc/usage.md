# CodingAgent 使用指南

## 快速开始

### 1. 配置 API 密钥

```bash
# 复制配置模板
cp .env.example .env

# 编辑 .env 文件，设置你的 API 密钥
nano .env
```

在 `.env` 文件中设置：
```bash
ANTHROPIC_API_KEY=sk-ant-your-actual-api-key-here
```

**获取 API Key:**
1. 访问 [Anthropic Console](https://console.anthropic.com/settings/keys)
2. 创建新的 API 密钥
3. 复制密钥到 `.env` 文件

### 2. 运行项目

```bash
# 加载环境变量
source .env

# 运行项目
cargo run
```

## 使用示例

启动后，你会看到：
```
You>
```

现在可以直接输入命令：

```
You> 你好
You> 列出当前目录的文件
You> 读取 README.md
You> 帮我写一个 Python 函数
You> 在 src/main.rs 中搜索 "main"
You> exit  # 退出程序
```

## 可用工具

- **bash** - 执行 shell 命令
- **read** - 读取文件内容
- **write** - 写入文件
- **glob** - 查找文件（支持模式匹配）
- **grep** - 搜索内容
- **edit** - 编辑文件

## 配置选项

在 `.env` 文件中可以配置：

```bash
# 选择模型
AGENT_MODEL=claude-sonnet-4-6        # 推荐
# AGENT_MODEL=claude-opus-4-6        # 最强
# AGENT_MODEL=claude-haiku-4-5-20251001  # 最快

# 会话配置
MAX_ROUNDS=50                        # 最大对话轮数
SESSION_DIR=./sessions               # 会话存储目录

# 日志调试
RUST_LOG=debug                       # 启用详细日志
```

## 开发命令

```bash
cd coding-agent
cargo build    # 编译
cargo test     # 测试
cargo run      # 运行
```

## 常见问题

### API 密钥错误
检查 `.env` 文件中的 `ANTHROPIC_API_KEY`

### 编译失败
确保使用新版本 Rust：`source "$HOME/.cargo/env"`

### 网络问题
已配置国内镜像，应该很快
