# 🤖 CodingAgent

一个基于 Rust 和 Tirea 框架的智能代码编辑助手。

## 🚀 快速开始

```bash
# 1. 配置 API 密钥
cp .env.example .env
# 编辑 .env 文件，设置你的 ANTHROPIC_API_KEY

# 2. 运行
./coding-agent/run.sh
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

## 🛠️ 功能特性

- **6 个核心工具**：bash、read、write、glob、grep、edit
- **交互式界面**：像与 Claude 一样对话
- **智能代码理解**：支持多种编程语言
- **文件操作**：读取、编辑、搜索文件

## 📋 详细文档

- [HOW_TO_USE.md](HOW_TO_USE.md) - 详细使用指南
- [SETUP.md](SETUP.md) - 配置说明
- [QUICKSTART.md](QUICKSTART.md) - 快速开始

## 📝 技术栈

- **Rust** - 系统编程语言
- **Tirea** - Agent 框架
- **Tokio** - 异步运行时
- **Anthropic API** - Claude 模型

## ⚠️ 注意事项

- 需要配置 Anthropic API 密钥
- 需要 Rust 1.82+ 版本
- 第一次运行会自动编译

## 📄 许可证

MIT License
