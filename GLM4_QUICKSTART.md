# GLM-4 快速启动指南

## 问题：为什么 Model 还是 `claude-sonnet-4-6`？

你的系统中已经设置了 `AGENT_MODEL=claude-sonnet-4-6` 环境变量，它会覆盖 `.cargo/config.toml` 的设置。

## 解决方案：使用启动脚本

我们创建了 `run-glm4.sh` 脚本来确保正确的配置。

### 方法 1：直接运行（推荐）

```bash
./run-glm4.sh 你的API密钥
```

### 方法 2：设置后运行

```bash
# 1. 设置 API key（一次性，保存到 ~/.bashrc）
echo 'export GLM4_API_KEY="你的API密钥"' >> ~/.bashrc
source ~/.bashrc

# 2. 运行
./run-glm4.sh
```

### 方法 3：手动设置环境变量

```bash
export OPENAI_API_KEY="你的API密钥"
export AGENT_MODEL="glm-4.7"
cargo run
```

> **注意**：不要设置 `OPENAI_BASE_URL`！代码会自动使用正确的 BigModel Coding endpoint。

## 验证配置

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
```

## 获取 API Key

访问 [BigModel.cn](https://open.bigmodel.cn/) 获取你的 API key。

## 故障排除

**问题**：Model 还是 `claude-sonnet-4-6`
- **解决**：确保设置了 `AGENT_MODEL=glm-4.7` 环境变量

**问题**：提示 "API key not set"
- **解决**：检查 API key 是否正确设置，运行 `echo $OPENAI_API_KEY` 查看

**问题**：认证失败
- **解决**：确认 API key 来自 BigModel.cn 且有效

**问题**：余额不足或无可用资源包
- **解决**：在 [BigModel.cn](https://open.bigmodel.cn/) 充值或购买资源包
