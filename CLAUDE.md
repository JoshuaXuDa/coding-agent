# CodingAgent

## 项目概述
基于 Rust 的 AI 编程助手，使用 tirea 框架 + GLM-4 模型 + ratatui TUI。
采用 DDD 架构，提供 15 个工具（文件操作、命令执行、Web 搜索等）。

## 构建与运行

```bash
# 设置 API Key
export OPENAI_API_KEY="your-key"

# 构建
cargo build --release

# 运行
cargo run --bin coding-agent

# 或使用启动脚本
./coding-agent.sh

# 运行测试
cargo test

# 最小构建（无 Web 工具）
cargo build --features minimal
```

## 架构

```
coding-agent/src/
├── main.rs              # 入口，仅 TUI 模式
├── config.rs            # 配置加载（agent.json + prompt.txt）
├── llm_logger.rs        # LLM 交互日志
├── tools/               # 工具上下文（核心）
│   ├── domain/          # 领域层（validation, json_builder, error_handler, registry）
│   └── application/     # 应用层（各工具实现）
├── platform/            # 跨平台抽象（filesystem, command）
├── ui/                  # TUI 界面（ratatui）
├── state/               # 状态管理
├── context/             # 文件引用注入（@filename）
├── behaviors/           # 行为（系统提示、输出过滤）
└── logging/             # 日志配置
```

## 代码约定

- 工具响应统一使用 **JSON 格式**（通过 `JsonBuilder`），不使用 XML
- 错误处理使用 `anyhow::Result`，工具错误使用 `ToolError`
- 静态正则使用 `std::sync::LazyLock<Regex>` 预编译
- 配置文件：`coding-agent/config/agent.json` 和 `config/prompt.txt`

## 关键配置

- API Key：通过 `OPENAI_API_KEY` 环境变量
- 模型：`AGENT_MODEL` 环境变量（默认 glm-4.7）
- 日志级别：`RUST_LOG` 环境变量
