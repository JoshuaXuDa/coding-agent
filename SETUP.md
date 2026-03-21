# GLM-4 API 配置指南

本指南介绍如何配置 coding-agent 使用智谱AI (BigModel.cn) 的 GLM-4 API。

## ⚠️ 安全提醒

**绝对不要**将 API key 直接写在 `.cargo/config.toml` 中，那样会被提交到 git 仓库！

请使用以下安全方式之一。

## 获取 API Key

1. 访问 [BigModel.cn](https://open.bigmodel.cn/)
2. 登录你的账号
3. 进入 API Keys 页面
4. 创建新的 API key

## 推荐配置方式

### 方式 1：.env.local 文件（最推荐）

1. 编辑项目根目录下的 `.env.local` 文件：
   ```bash
   OPENAI_API_KEY="你的实际API密钥"
   ```

2. 运行前加载环境变量：
   ```bash
   source .env.local
   cargo run
   ```

### 方式 2：一次性设置环境变量

```bash
export OPENAI_API_KEY="你的实际API密钥"
cargo run
```

### 方式 3：添加到 shell profile

将以下内容添加到 `~/.bashrc` 或 `~/.zshrc`：

```bash
# GLM-4 API Configuration (BigModel.cn)
export OPENAI_API_KEY="你的实际API密钥"
```

然后重新加载：
```bash
source ~/.bashrc   # 或 source ~/.zshrc
```

## 验证配置

设置完成后，运行 `cargo run` 应该会看到：

```
🤖 CodingAgent starting...
📦 Model: glm-4.7
🌐 Base URL: https://open.bigmodel.cn/api/paas/v4
📁 Session directory: ./sessions

✅ Registered 6 tools:
   - bash
   - edit
   - glob
   - grep
   - read
   - write
```

## 切换不同 API

```bash
# 使用 GLM-4
export OPENAI_BASE_URL="https://open.bigmodel.cn/api/paas/v4"
export AGENT_MODEL="glm-4.7"

# 使用 OpenAI
export OPENAI_BASE_URL="https://api.openai.com/v1"
export AGENT_MODEL="gpt-4"

# 使用 DeepSeek
export OPENAI_BASE_URL="https://api.deepseek.com"
export AGENT_MODEL="deepseek-chat"
```

## GLM-4 模型选项

- `glm-4.7` - 最新 GLM-4.7 模型（推荐）
- `glm-4` - 标准 GLM-4 模型
- `glm-4-flash` - 更快响应的模型
- `glm-4-plus` - 增强能力模型

## API 规格说明

GLM-4 API 使用 OpenAI 兼容格式：

### Coding Endpoint（推荐用于代码生成）

- **端点**: `https://open.bigmodel.cn/api/coding/paas/v4/chat/completions`
- **认证**: `Authorization: Bearer <api-key>`
- **流式传输**: 支持 `stream: true`
- **最大 tokens**: 65536
- **温度范围**: 0.0-2.0
- **适配器**: 使用 ZAI adapter（GLM 模型的原生适配器）

### 标准 Endpoint

- **端点**: `https://open.bigmodel.cn/api/paas/v4/chat/completions`
- 注意：此 endpoint 可能需要不同的资源包

## 重要提示

1. **不要设置 OPENAI_BASE_URL 环境变量**：代码通过 `client_resolver.rs` 自动配置正确的 endpoint
2. **只要设置 OPENAI_API_KEY**：endpoint 会在代码中自动设置为 coding endpoint
3. **余额要求**：确保您的 BigModel.cn 账户有足够的余额或有效的资源包

## 故障排除

**问题**：显示 "API key not set"
**解决**：确保 `OPENAI_API_KEY` 环境变量已设置，运行 `echo $OPENAI_API_KEY` 检查

**问题**：认证失败
**解决**：检查 API key 是否正确，确保从 BigModel.cn 获取的是有效密钥

**问题**：连接超时
**解决**：检查网络连接，确保可以访问 `open.bigmodel.cn`
