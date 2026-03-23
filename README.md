# 🤖 CodingAgent

基于 Rust 和 Tirea 框架的智能代码编辑助手，使用 GLM-4 (智谱清言) 模型。

## 🚀 快速开始

### 1. 获取 API Key

访问 [BigModel.cn](https://open.bigmodel.cn/) 获取你的 API key。

### 2. 配置

CodingAgent 现在使用 JSON 配置文件进行配置，无需自定义代码。

**配置文件位置**: `coding-agent/config/agent.json`

```json
{
  "providers": {
    "bigmodel-coding": {
      "endpoint": "https://open.bigmodel.cn/api/coding/paas/v4/",
      "auth": { "kind": "env", "name": "OPENAI_API_KEY" },
      "adapter_kind": "openai"
    }
  },
  "models": {
    "glm": { "provider": "bigmodel-coding", "model": "GLM-4.5-air" }
  },
  "agents": [{
    "kind": "local",
    "id": "coding-agent",
    "model": "glm",
    "system_prompt": "...",
    "max_rounds": 50
  }]
}
```

**环境变量**:
```bash
# 设置 API Key
export OPENAI_API_KEY="你的实际API密钥"

# 可选：覆盖模型（默认使用配置文件中的设置）
export AGENT_MODEL="glm-4.7"
```

### 3. 运行

```bash
cd coding-agent
cargo run --release
```

## ✅ 验证配置

运行后应该看到：

```
🤖 CodingAgent starting...
✅ Registered 9 tools:
   - bash
   - edit
   - glob
   - grep
   - head_tail
   - list
   - read
   - stat
   - write

📝 LLM interaction logging enabled (logs/llm_interactions.log)

📝 Loading configuration...
✅ Agent: coding-agent
✅ AgentOS initialized successfully

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
| **list** | 列出目录内容 | 列出 `src/` 目录文件 |
| **stat** | 获取文件信息 | 查看 `Cargo.toml` 详细信息 |
| **head_tail** | 预览文件首尾 | 查看文件前 10 行或后 10 行 |

## 📊 LLM 交互日志

CodingAgent 现在支持 LLM 交互日志记录，所有对话、工具调用和响应都会被记录到 `logs/llm_interactions.log`。

**日志格式** (JSON):
```json
{
  "timestamp": "2026-03-23T10:30:45Z",
  "type": "request|response|tool_call|error",
  "round": 1,
  "user_message": "...",
  "llm_response": "...",
  "duration_ms": 1234
}
```

**查看日志**:
```bash
cat logs/llm_interactions.log
tail -f logs/llm_interactions.log
```

## ⚙️ 配置选项

### JSON 配置文件

配置文件 `coding-agent/config/agent.json` 支持：

#### Providers (提供商)

```json
{
  "providers": {
    "bigmodel-coding": {
      "endpoint": "https://open.bigmodel.cn/api/coding/paas/v4/",
      "auth": { "kind": "env", "name": "OPENAI_API_KEY" },
      "adapter_kind": "openai"
    },
    "deepseek": {
      "endpoint": "https://api.deepseek.com",
      "auth": { "kind": "env", "name": "DEEPSEEK_API_KEY" }
    }
  }
}
```

- `endpoint`: API 端点 URL
- `auth.kind`: 认证方式 (`env` 或 `token`)
- `auth.name`: 环境变量名 (当 `kind: env`)
- `adapter_kind`: 适配器类型 (`openai`, `anthropic`, 等)

#### Models (模型)

```json
{
  "models": {
    "glm": { "provider": "bigmodel-coding", "model": "GLM-4.5-air" },
    "gpt-4": { "provider": "openai", "model": "gpt-4-turbo" }
  }
}
```

#### Agents (代理)

```json
{
  "agents": [{
    "kind": "local",
    "id": "coding-agent",
    "model": "glm",
    "system_prompt": "...",
    "max_rounds": 50
  }]
}
```

### GLM-4 模型选项

在配置文件中修改 `model` 字段：

- `GLM-4.5-air` - 轻量级模型（默认）
- `GLM-4.5` - 标准模型
- `glm-4-plus` - 增强能力
- `glm-4-flash` - 更快响应

## 🔧 技术栈

- **Rust** - 系统编程语言
- **Tirea** - Agent 框架 (支持 JSON 配置)
- **GLM-4** - 智谱 AI 大语言模型
- **Tokio** - 异步运行时

## 📝 API 配置详情

### BigModel Coding Endpoint

- **端点**: `https://open.bigmodel.cn/api/coding/paas/v4/`
- **适配器**: OpenAI 兼容
- **流式传输**: 支持

### 多提供商配置示例

```json
{
  "providers": {
    "bigmodel-coding": {
      "endpoint": "https://open.bigmodel.cn/api/coding/paas/v4/",
      "auth": { "kind": "env", "name": "OPENAI_API_KEY" },
      "adapter_kind": "openai"
    },
    "openai": {
      "endpoint": "https://api.openai.com/v1",
      "auth": { "kind": "env", "name": "OPENAI_API_KEY" }
    },
    "deepseek": {
      "endpoint": "https://api.deepseek.com",
      "auth": { "kind": "env", "name": "DEEPSEEK_API_KEY" }
    }
  },
  "models": {
    "glm": { "provider": "bigmodel-coding", "model": "GLM-4.5-air" },
    "gpt-4": { "provider": "openai", "model": "gpt-4-turbo" },
    "deepseek-chat": { "provider": "deepseek", "model": "deepseek-chat" }
  },
  "agents": [{
    "id": "coding-agent",
    "model": "glm",
    "system_prompt": "..."
  }]
}
```

## ❓ 常见问题

### 提示 "API key not set"

确保设置了环境变量：
```bash
export OPENAI_API_KEY="你的API密钥"
```

### 认证失败

确认 API key 有效，且配置文件中的 `endpoint` 和 `adapter_kind` 正确。

### 编译失败

确保使用新版本 Rust（1.70+）：
```bash
source "$HOME/.cargo/env"
rustc --version
```

### 配置文件加载失败

检查 `coding-agent/config/agent.json` 是否存在且 JSON 格式正确。

## ⚠️ 安全提醒

**绝对不要**将 API key 直接写在配置文件中或提交到 git 仓库！

使用环境变量来存储敏感信息：
```json
{
  "auth": { "kind": "env", "name": "OPENAI_API_KEY" }
}
```

## 📄 许可证

MIT License
