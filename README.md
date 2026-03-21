# 🤖 CodingAgent

基于 Rust 和 Tirea 框架的智能代码编辑助手，使用 GLM-4 (智谱清言) 模型。

## 🚀 快速开始

### 1. 获取 API Key

访问 [BigModel.cn](https://open.bigmodel.cn/) 获取你的 API key。

### 2. 配置 API Key

**方式 1：使用 .env.local 文件（推荐）**

```bash
# 复制配置模板
cp .env.local.example .env.local

# 编辑 .env.local，设置你的 API key
echo 'OPENAI_API_KEY="你的实际API密钥"' > .env.local
```

**方式 2：一次性环境变量**

```bash
export OPENAI_API_KEY="你的实际API密钥"
export AGENT_MODEL="glm-4.7"
```

**方式 3：添加到 shell profile**

```bash
echo 'export OPENAI_API_KEY="你的实际API密钥"' >> ~/.bashrc
echo 'export AGENT_MODEL="glm-4.7"' >> ~/.bashrc
source ~/.bashrc
```

### 3. 运行

```bash
# 加载环境变量并运行
source .env.local
cargo run --release

# 或者直接运行（如果已设置到 ~/.bashrc）
cargo run --release
```

## ✅ 验证配置

运行后应该看到：

```
🤖 CodingAgent starting...
📦 Model: glm-4.7         ✅ 正确！
📁 Session directory: ./sessions

✅ Registered 6 tools:
   - bash
   - edit
   - glob
   - grep
   - read
   - write

═══════════════════════════════════════════════════════════
  CodingAgent Ready - Type your message below
═══════════════════════════════════════════════════════════

You>
```

## 💬 使用示例

启动后，直接输入命令：

```
You> 你好
You> 列出当前目录的文件
You> 读取 README.md
You> 帮我写一个快速排序函数
You> 在 src/main.rs 中搜索 "main"
You> exit
```

## 🛠️ 可用工具

| 工具 | 功能 | 示例 |
|------|------|------|
| **bash** | 执行 shell 命令 | `cargo build`、`npm test`、`git status` |
| **read** | 读取文件内容 | `src/main.rs`、`README.md` |
| **write** | 写入文件 | 创建新文件或完全替换 |
| **edit** | 编辑文件 | 精准修改现有文件 |
| **glob** | 查找文件 | `**/*.rs`、`src/**/*.json` |
| **grep** | 搜索内容 | `fn\s+\w+`、`TODO` |

## ⚙️ 配置选项

### 模型选择

```bash
# GLM-4 模型选项
export AGENT_MODEL="glm-4.7"        # 最新 GLM-4.7（推荐）
export AGENT_MODEL="glm-4"          # 标准 GLM-4
export AGENT_MODEL="glm-4-flash"    # 更快响应
export AGENT_MODEL="glm-4-plus"     # 增强能力
```

### 会话配置

```bash
MAX_ROUNDS=50              # 最大对话轮数
SESSION_DIR=./sessions     # 会话存储目录
RUST_LOG=debug            # 启用详细日志
```

## 🔧 技术栈

- **Rust** - 系统编程语言
- **Tirea** - Agent 框架
- **GLM-4** - 智谱 AI 大语言模型
- **Tokio** - 异步运行时

## 📝 API 配置详情

### BigModel Coding Endpoint

- **端点**: `https://open.bigmodel.cn/api/coding/paas/v4/chat/completions`
- **认证**: `Authorization: Bearer <api-key>`
- **适配器**: OpenAI 兼容
- **流式传输**: 支持

### 切换不同 API

```bash
# 使用 GLM-4（默认）
export OPENAI_API_KEY="你的GLM密钥"
export AGENT_MODEL="glm-4.7"

# 使用 OpenAI
export OPENAI_API_KEY="你的OpenAI密钥"
export OPENAI_BASE_URL="https://api.openai.com/v1"
export AGENT_MODEL="gpt-4"

# 使用 DeepSeek
export OPENAI_API_KEY="你的DeepSeek密钥"
export OPENAI_BASE_URL="https://api.deepseek.com"
export AGENT_MODEL="deepseek-chat"
```

## ❓ 常见问题

### Model 还是 `claude-sonnet-4-6`？

确保设置了 `AGENT_MODEL=glm-4.7` 环境变量。

### 提示 "API key not set"

检查 API key 是否正确设置：
```bash
echo $OPENAI_API_KEY
```

### 认证失败

确认 API key 来自 BigModel.cn 且有效。

### 余额不足

在 [BigModel.cn](https://open.bigmodel.cn/) 充值或购买资源包。

### 编译失败

确保使用新版本 Rust（1.82+）：
```bash
source "$HOME/.cargo/env"
rustc --version
```

## ⚠️ 安全提醒

**绝对不要**将 API key 直接写在代码中或提交到 git 仓库！

使用 `.env.local` 文件（已在 `.gitignore` 中）来存储敏感信息。

## 📄 许可证

MIT License
