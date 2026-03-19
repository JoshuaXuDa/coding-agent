CodingAgent：基于 Tirea 框架的 MVP 实现计划
Context
用户要基于 tirea 框架（samples/tirea 参考实现）开发一个 Coding Agent，作为 tirea 的一个子 Agent / 应用。

目标：让 tirea 框架驱动一个能够读写代码库、执行命令、理解代码的编程助手 Agent。

技术栈约束：

语言：Rust（与 tirea 框架一致）
框架：tirea v0.4.0（已发布到 crates.io，直接版本依赖，无需 path 依赖或编译 .so）
模型：Claude claude-sonnet-4-6（通过 genai 或 Anthropic SDK）
项目位置：/Users/xudagao/code/code-agent/ 根目录
关键发现：cargo search tirea 显示 tirea 及其所有子 crate 均已发布到 crates.io：

tirea = "0.4.0" — 核心框架
tirea-agent-loop = "0.3.0" — LLM 推理引擎
tirea-extension-permission = "0.4.0" — 权限控制扩展
还有 12+ 个相关 crate
这意味着：完全不需要引用 samples/tirea 的源码，直接在 Cargo.toml 中写版本号依赖即可。samples/tirea 仅用于阅读架构设计参考。

方案设计
核心思路
基于 tirea 的开发模式，实现三类核心组件：

State（状态）：CodingState，保存当前工作目录、打开的文件、TodoList、已执行命令历史
Tools（工具）：6 个核心编码工具，每个实现 tirea 的 Tool trait
AgentBehavior（行为）：注入系统提示、权限检查、输出截断保护
最后用 AgentOsBuilder 拼装，通过 AG-UI 或 CLI 协议运行。

项目目录结构
code-agent/
├── Cargo.toml                    # Rust workspace（引用 tirea 为本地依赖）
├── Cargo.lock
│
├── coding-agent/                 # 主 crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # 入口：构建并启动 AgentOs
│       │
│       ├── state/
│       │   ├── mod.rs
│       │   ├── coding_state.rs   # CodingState 定义 + #[derive(State)]
│       │   └── actions.rs        # CodingAction 枚举（状态变更动作）
│       │
│       ├── tools/
│       │   ├── mod.rs            # 工具注册表（返回 HashMap<String, Arc<dyn Tool>>）
│       │   ├── read.rs           # ReadTool：读取文件内容（支持行范围）
│       │   ├── write.rs          # WriteTool：写整个文件
│       │   ├── edit.rs           # EditTool：字符串精确替换（多路匹配策略）
│       │   ├── glob.rs           # GlobTool：文件名 glob 模式匹配
│       │   ├── grep.rs           # GrepTool：正则内容搜索（ripgrep 包装）
│       │   └── bash.rs           # BashTool：Shell 命令执行（超时控制）
│       │
│       ├── behaviors/
│       │   ├── mod.rs
│       │   ├── system_prompt.rs  # SystemPromptBehavior：before_inference 注入系统提示
│       │   └── output_guard.rs   # OutputGuardBehavior：after_tool_execute 截断超大输出
│       │
│       └── prompt.rs             # 系统提示内容（专业编程 Agent 角色定义）
│
└── samples/                      # 参考代码（只读，不修改）
    ├── opencode/
    └── tirea/
状态设计（CodingState）
// coding-agent/src/state/coding_state.rs

#[derive(Debug, Clone, Default, Serialize, Deserialize, State)]
#[tirea(action = "CodingAction")]
pub struct CodingState {
    /// 当前工作目录
    pub working_dir: Option<String>,

    /// TodoList（任务进度跟踪）
    pub todos: Vec<TodoItem>,

    /// 最近执行的命令历史（最多保留 20 条）
    pub command_history: Vec<CommandRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodingAction {
    SetWorkingDir(String),
    AddTodo(TodoItem),
    UpdateTodoStatus { id: String, status: TodoStatus },
    AddCommandRecord(CommandRecord),
}
工具设计（6 个核心工具）
工具	功能	关键实现点
read	读取文件（支持 offset/limit 行范围）	最大 2000 行截断，超出提示使用 offset
write	写整个文件	先检查父目录存在
edit	字符串替换（old → new）	3 路策略：精确匹配 → 行 trim 匹配 → 空白归一化匹配；全失败报错
glob	文件名模式匹配	使用 glob crate，结果按修改时间排序，最多 100 个
grep	正则内容搜索	包装 grep 命令或 regex crate，返回文件路径:行号:内容
bash	执行 shell 命令	30s 超时，输出截断 50KB，记录到 command_history
所有工具实现 tirea 的 Tool trait：

descriptor()：返回工具名称、描述和 JSON Schema 参数定义
execute_effect()：执行副作用，返回 ToolExecutionEffect（含 ToolResult 和可选的 AnyStateAction）
输出截断规范（借鉴 opencode 逻辑）：

单次工具输出 > 50KB 或 > 2000 行 → 截断并在末尾附注 "Output truncated. Use offset/limit parameters to read specific sections."
行为设计（AgentBehavior）
SystemPromptBehavior（before_inference）
在每次推理前注入系统提示，包含：

角色定义：资深软件工程师助手
工作规范：读文件前先探索，最小必要修改，不过度工程化
工具使用指南：何时用 glob vs grep，edit 的使用前提（必须先 read）
OutputGuardBehavior（after_tool_execute）
检测工具输出是否超过阈值
超出时截断并追加提示（已在 Tool 层处理，Behavior 层作为双重保险）
入口装配（main.rs）
// coding-agent/src/main.rs

#[tokio::main]
async fn main() {
    let agent_def = AgentDefinition {
        id: "coding-agent".into(),
        model: "claude-sonnet-4-6".into(),   // 或 CODING_AGENT_MODEL 环境变量
        system_prompt: prompt::SYSTEM_PROMPT.into(),
        max_rounds: 50,
        behavior_ids: vec!["system_prompt".into(), "output_guard".into()],
        ..Default::default()
    };

    let tools = tools::build_tool_map();   // 返回 HashMap<String, Arc<dyn Tool>>

    let os = AgentOsBuilder::new()
        .with_agent_spec(AgentDefinitionSpec::local(agent_def))
        .with_tools(tools)
        .with_registered_behavior("system_prompt", Arc::new(SystemPromptBehavior))
        .with_registered_behavior("output_guard", Arc::new(OutputGuardBehavior))
        .with_agent_state_store(Arc::new(FileStore::new("./sessions")))
        .build()
        .unwrap();

    // 根据命令行参数决定运行模式
    // --cli: 简单 stdin/stdout 交互
    // --server: 启动 HTTP 服务（AG-UI 协议）
    run_mode(os).await;
}
关键参考文件（samples/tirea，仅逻辑参考）
samples/tirea/examples/ - 工具注册和 AgentOsBuilder 装配示例
samples/tirea/crates/tirea-contract/src/ - Tool trait 和 AgentBehavior trait 定义
samples/tirea/crates/tirea-agentos/ - AgentOsBuilder 装配和运行时
samples/opencode/packages/opencode/src/tool/ - 工具实现逻辑（edit 的多路 Replacer，bash 的超时，输出截断）
MVP 实现顺序
初始化 Workspace

创建根 Cargo.toml（workspace）
添加 coding-agent crate
配置 tirea 依赖（本地路径指向 samples/tirea，或确认发布版本）
State 层

实现 CodingState + CodingAction
实现 reduce 函数
Tools 层（从简单到复杂）

GlobTool → GrepTool → ReadTool → WriteTool → BashTool → EditTool（最复杂，含多路匹配）
Behaviors 层

SystemPromptBehavior（before_inference 注入 prompt）
OutputGuardBehavior（after_tool_execute 截断保护）
装配与运行

main.rs 中用 AgentOsBuilder 组装
实现简单的 CLI 交互模式（stdin/stdout）
验证：向 Agent 发送任务，观察工具调用和响应
验证方式
基本功能测试
# 启动 Agent，发送一个实际编程任务
cargo run -- --cli

# 输入：请帮我查找 src/ 目录下所有的 Rust 文件，然后显示 main.rs 的内容
# 预期：Agent 先调用 glob 工具，再调用 read 工具，给出准确回答
工具单元测试
# 为每个工具编写集成测试
cargo test -p coding-agent tools::

# EditTool 重点测试三路匹配策略
# BashTool 重点测试超时控制
端到端测试
向 Agent 发送一个真实的编程任务：

"在这个项目中添加一个 hello_world 函数到 src/lib.rs"
预期 Agent 会 glob → read → edit → bash（运行测试）完成任务
关键风险与决策
风险 1（已解决）：tirea 的依赖引入方式

tirea v0.4.0 已发布到 crates.io，直接版本依赖
samples/tirea 仅作阅读参考，不引入源码，符合项目规则
Cargo.toml 核心依赖：

[dependencies]
tirea = "0.4.0"
tirea-agent-loop = "0.3.0"
tirea-extension-permission = "0.4.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
glob = "0.3"          # GlobTool
regex = "1"           # GrepTool
模型配置（支持 Claude / GLM 等任意 OpenAI-compatible 模型）

tirea 底层使用 genai crate，支持通过环境变量或代码级 Resolver 切换模型：

方案 A：环境变量（MVP 阶段，推荐）

# 使用 Claude
export ANTHROPIC_API_KEY="sk-ant-..."
export AGENT_MODEL="claude-sonnet-4-6"

# 或切换为 GLM（OpenAI 兼容接口）
export OPENAI_API_BASE="https://open.bigmodel.cn/api/paas/v4/"
export OPENAI_API_KEY="你的智谱 API Key"
export AGENT_MODEL="glm-4"
方案 B：代码级配置（后续迭代） 通过 genai::Client::builder().with_service_target_resolver_fn() 显式指定 endpoint + auth，支持配置文件驱动多模型切换。

设计原则：AgentDefinition::model 从环境变量 AGENT_MODEL 读取，默认 "claude-sonnet-4-6"。

决策：Edit 工具简化

MVP 阶段先实现精确匹配 + 行 trim 匹配两路，Levenshtein 模糊匹配留待后续迭代
风险 3：tirea v0.4.0 API 与 samples/tirea 的差异

crates.io 上的版本可能与 samples/tirea（开发版）有轻微 API 差异
以 crates.io 发布版的文档为准，samples 仅参考设计理念
