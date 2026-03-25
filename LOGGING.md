# 日志系统使用指南

## 概述

CodingAgent 现在拥有一个统一、优雅的日志系统，支持：

- 标准日志宏（`debug!`, `info!`, `warn!`, `error!`）
- 彩色输出和时间戳
- 日志级别控制
- TUI 调试面板
- 配置文件和环境变量支持

## 基本使用

### 在代码中使用日志宏

```rust
use log::{info, debug, warn, error};

// 信息日志
info!("Application started successfully");
info!("Registered {} tools", tool_count);

// 调试日志
debug!("Processing request with ID: {}", request_id);
debug!("Command: {:?}, Args: {:?}", command, args);

// 警告日志
warn!("Using default configuration");
warn!("Failed to load file: {}, using defaults", path);

// 错误日志
error!("Failed to connect to service: {}", error);
error!("Critical error: {}", err);
```

## 配置

### 通过环境变量配置

```bash
# 设置日志级别
RUST_LOG=info                    # 全局级别
RUST_LOG=debug,coding-agent::tools=info  # 模块特定级别

# 启用/禁用彩色输出
CODING_AGENT_LOG_COLORED=true    # 启用（默认）
CODING_AGENT_LOG_COLORED=false   # 禁用

# 显示时间戳
CODING_AGENT_LOG_TIMESTAMP=true  # 显示（默认）
CODING_AGENT_LOG_TIMESTAMP=false # 隐藏

# 显示模块路径
CODING_AGENT_LOG_MODULE=true     # 显示
CODING_AGENT_LOG_MODULE=false    # 隐藏（默认）

# 使用 emoji 指示器
CODING_AGENT_LOG_EMOJI=true      # 启用
CODING_AGENT_LOG_EMOJI=false     # 禁用（默认）

# 启用 TUI 调试面板
CODING_AGENT_TUI_DEBUG=true      # 启动时显示调试面板
```

### 通过配置文件配置

编辑 `config/logging.toml`:

```toml
[general]
default_level = "info"
colored = true
show_timestamp = true
show_module = false
use_emoji = false

[outputs.console]
enabled = true
level = "info"

[tui]
log_to_panel = true
max_log_lines = 1000
show_debug_panel = false
```

## TUI 调试面板

### 快捷键

- `F12` - 切换调试面板显示/隐藏
- `l` - 循环切换日志级别（OFF → ERROR → WARN → INFO → DEBUG → TRACE → OFF）
- `c` - 清除日志缓冲区
- `PageUp/PageDown` - 上下翻页
- `↑/↓` - 逐行滚动

### 调试面板功能

- 显示最多 1000 条日志（可配置）
- 支持日志级别过滤
- 自动滚动到最新日志
- 彩色显示不同级别日志

## 日志级别说明

| 级别 | 用途 | 示例 |
|------|------|------|
| ERROR | 错误信息，影响功能 | "Failed to execute command" |
| WARN | 警告信息，不影响功能 | "Using default configuration" |
| INFO | 一般信息 | "Application started" |
| DEBUG | 调试信息 | "Processing request" |
| TRACE | 详细跟踪信息 | "Function entry/exit" |

## 迁移指南

### 从 `println!`/`eprintln!` 迁移

**之前：**
```rust
println!("✅ Registered {} tools:", tools.len());
eprintln!("⚠️  File not found: {}", path);
```

**之后：**
```rust
info!("Registered {} tools", tools.len());
warn!("File not found: {}", path);
```

### 映射规则

| 原调用 | 新调用 | 说明 |
|--------|--------|------|
| `println!("✅ ...")` | `info!(...)` | 成功信息 |
| `eprintln!("⚠️ ...")` | `warn!(...)` | 警告信息 |
| `eprintln!("❌ ...")` | `error!(...)` | 错误信息 |
| `eprintln!("[DEBUG] ...")` | `debug!(...)` | 调试信息 |
| `println!("🔄 ...")` | `debug!(...)` | 进度信息 |

## 示例

### 运行应用

```bash
# 使用默认配置
cargo run

# 启用调试日志
RUST_LOG=debug cargo run

# 只显示错误和警告
RUST_LOG=warn cargo run

# 某些模块使用调试级别，其他使用信息级别
RUST_LOG=info,coding-agent::tools=debug,coding-agent::ui=debug cargo run
```

### 输出示例

```
[14:30:45] INFO  CodingAgent starting...
[14:30:45] INFO  Registered 6 tools
[14:30:45] INFO   - bash
[14:30:45] INFO   - edit
[14:30:45] INFO   - glob
[14:30:45] INFO   - grep
[14:30:45] INFO   - list
[14:30:45] INFO   - read
[14:30:46] INFO  AgentOS initialized successfully
[14:30:46] INFO  Starting TUI Mode...
```

## 技术细节

### 日志库

- **log**: 标准日志接口
- **env_logger**: 日志实现，支持环境变量配置
- **termcolor**: 终端彩色输出
- **chrono**: 时间戳生成

### 架构

```
log macros (debug!, info!, etc.)
         ↓
   env_logger
         ↓
   Custom Formatter
         ↓
    Terminal/File
```

### TUI 日志流程

```
log macros
         ↓
   env_logger (console)
         ↓
   Log Bridge (optional)
         ↓
   TUI Debug Panel
```

## 故障排查

### 日志没有输出

检查 `RUST_LOG` 环境变量：
```bash
export RUST_LOG=debug
cargo run
```

### TUI 调试面板没有显示

1. 按 `F12` 切换显示
2. 检查日志级别过滤（按 `l` 切换）
3. 确保有日志产生

### 彩色输出不工作

检查终端支持：
```bash
# 禁用彩色输出
CODING_AGENT_LOG_COLORED=false cargo run
```

## 未来改进

- [ ] 日志文件轮转
- [ ] 结构化日志（JSON 格式）
- [ ] 日志聚合和分析
- [ ] 性能监控日志
- [ ] 分布式跟踪支持
