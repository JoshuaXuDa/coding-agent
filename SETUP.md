# CodingAgent 配置指南

## 快速开始

### 1. 配置 API 密钥

```bash
# 复制配置模板
cp .env.example .env

# 编辑 .env 文件，设置你的 API 密钥
nano .env  # 或使用你喜欢的编辑器
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

**方式1：使用启动脚本（推荐）**
```bash
./run.sh
```

**方式2：手动设置环境变量**
```bash
# 加载环境变量
source .env

# 加载 Rust 环境（如果使用 rustup）
source "$HOME/.cargo/env"

# 运行项目
cargo run
```

**方式3：直接设置环境变量**
```bash
export ANTHROPIC_API_KEY="your-api-key"
export AGENT_MODEL="claude-sonnet-4-6"
cargo run
```

## 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 | 必填 |
|--------|------|--------|------|
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | - | ✅ |
| `AGENT_MODEL` | 使用的模型 | `claude-sonnet-4-6` | ❌ |
| `MAX_ROUNDS` | 最大推理轮数 | `50` | ❌ |
| `SESSION_DIR` | 会话存储目录 | `./sessions` | ❌ |
| `RUST_LOG` | 日志级别 | - | ❌ |

### 可用模型

- `claude-sonnet-4-6` - 默认，平衡性能和速度
- `claude-opus-4-6` - 最强性能，速度较慢
- `claude-haiku-4-5-20251001` - 最快速度，性能略低

### 日志调试

启用详细日志：
```bash
# 在 .env 中添加
RUST_LOG=debug

# 或运行时设置
RUST_LOG=debug cargo run
```

## 项目结构

```
coding-agent/
├── coding-agent/          # 主要源代码
│   ├── src/
│   │   ├── main.rs       # 入口文件
│   │   ├── tools/        # 工具实现
│   │   ├── state/        # 状态管理
│   │   └── behaviors/    # 行为定义
│   └── Cargo.toml        # Rust 依赖配置
├── .env                   # 环境变量配置（不提交）
├── .env.example           # 配置模板
├── run.sh                 # 启动脚本
└── SETUP.md              # 本文档
```

## 常见问题

### Q: 提示 "ANTHROPIC_API_KEY not set"
**A:** 请确保 `.env` 文件存在且包含有效的 API 密钥。

### Q: 编译失败或版本错误
**A:** 确保 Rust 版本 >= 1.82.0：
```bash
rustc --version
source "$HOME/.cargo/env"
cargo build
```

### Q: 网络超时或下载慢
**A:** 已配置国内镜像（rsproxy.cn），如仍有问题检查网络连接。

### Q: 如何更换模型？
**A:** 在 `.env` 文件中修改 `AGENT_MODEL` 变量。

## 开发说明

### 编译项目
```bash
cargo build
```

### 运行测试
```bash
cargo test
```

### 检查代码
```bash
cargo check
```

### 格式化代码
```bash
cargo fmt
```

### 运行 linter
```bash
cargo clippy
```

## 相关资源

- [Anthropic API 文档](https://docs.anthropic.com/)
- [Tirea 框架文档](https://github.com/tirea-framework/tirea)
- [Rust 官方文档](https://www.rust-lang.org/learn)

## 许可证

MIT License
