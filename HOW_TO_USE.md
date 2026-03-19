# 🚀 CodingAgent 使用指南

## 最简单的使用方式

在项目根目录下运行：
```bash
./coding-agent/run.sh
```

就这么简单！

## 💬 使用示例

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

## 🛠️ 可用工具

- **bash** - 执行 shell 命令
- **read** - 读取文件内容
- **write** - 写入文件
- **glob** - 查找文件（支持模式匹配）
- **grep** - 搜索内容
- **edit** - 编辑文件

## ⚙️ 配置

编辑 `.env` 文件：
```bash
ANTHROPIC_API_KEY=你的API密钥
AGENT_MODEL=claude-sonnet-4-6
```

## 📝 其他启动方式

### 方式1：快速启动（推荐）
```bash
./coding-agent/run.sh
```

### 方式2：使用 cargo
```bash
source .env
cd coding-agent && cargo run
```

### 方式3：运行编译好的二进制
```bash
cd coding-agent
cargo build
./target/debug/coding-agent
```

## 🔧 开发命令

```bash
cd coding-agent
cargo build    # 编译
cargo test     # 测试
cargo run      # 运行
```

## ⚠️ 注意事项

- 确保已配置 `.env` 文件中的 `ANTHROPIC_API_KEY`
- 第一次运行会自动编译
- 输入 `exit`、`quit` 或 `q` 退出程序
- 需要安装 Rust 1.82+ 版本

## 🐛 遇到问题？

1. **API 密钥错误** → 检查 `.env` 文件
2. **编译失败** → 确保使用新版本 Rust：`source "$HOME/.cargo/env"`
3. **找不到命令** → 确保在项目根目录运行

## 📚 更多信息

- [SETUP.md](SETUP.md) - 详细配置指南
- [QUICKSTART.md](QUICKSTART.md) - 快速开始
