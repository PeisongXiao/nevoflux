# Native Agent 实现设计

> 版本: 1.0.0
> 日期: 2026-01-18
> 状态: 设计完成，待实现

---

## 1. 概述

本文档定义 NevoFlux Native Agent 的实现方案，基于 `2026-01-18-extension-native-agent-interface-design.md` 设计文档，实现 4 通道架构和 10 项核心功能。

### 1.1 核心功能列表

| # | 功能 | 优先级 | 说明 |
|---|------|--------|------|
| 1 | LLM 调用 | P0 | 4种模式：API/本地/账户订阅/页面模式 |
| 2 | Agentic Loop | P1 | 工具循环执行 |
| 3 | MCP Client | P1 | 调用其他 MCP 服务 |
| 4 | MCP Server | P2 | 对外暴露 Browser Use 能力 |
| 5 | 本地文件/脚本访问 | P1 | 需授权 |
| 6 | Browser Use API | P1 | 通过 Extension 调用 |
| 7 | Skills 系统 | P3 | 技能管理 |
| 8 | 插件系统 | P3 | WASM Sandbox |
| 9 | A2A 协议 | P3 | Agent-to-Agent 通信 |
| 10 | Human in the Loop | P0 | 权限请求授权 |

### 1.2 实现阶段

| 阶段 | 功能 | 理由 |
|------|------|------|
| **P0** | 协议升级 + 4通道架构 | 基础设施，所有功能依赖于此 |
| **P1** | Agentic Loop 完善 | 核心能力，用户可见价值最高 |
| **P2** | MCP Server | 对外暴露 Browser Use，与 Claude Code 等集成 |
| **P3** | 其他功能 | Skills、插件、A2A 等后续迭代 |

---

## 2. 架构设计

### 2.1 目标架构

```
┌─────────────────────────────────────────────────────────────┐
│ 目标：4 个 Native Messaging 连接，各自独立处理               │
│                                                             │
│   Extension                                                 │
│     │                                                       │
│     ├─ Port1 (com.nevoflux.agent.input)                    │
│     │    └──> InputChannelHandler ──> AgentCore            │
│     │                                                       │
│     ├─ Port2 (com.nevoflux.agent.output)                   │
│     │    <── OutputChannelHandler <── AgentCore            │
│     │                                                       │
│     ├─ Port3 (com.nevoflux.agent.mcp)                      │
│     │    <──> McpChannelHandler <──> McpServer             │
│     │                                                       │
│     └─ Port4 (com.nevoflux.agent.pagellm)                  │
│          <──> PageLlmChannelHandler <──> PageLlmAdapter    │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 AgentCore 组件

```
┌─────────────────────────────────────────────────────────────┐
│                        AgentCore                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │SessionManager│  │ LlmManager  │  │ ToolManager │         │
│  │  (会话管理)  │  │ (LLM调度)   │  │ (工具执行)  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ McpManager  │  │SkillManager │  │ PermManager  │         │
│  │ (MCP服务)   │  │ (技能管理)  │  │ (权限管理)  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                     Message Router                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Channel1 │ │ Channel2 │ │ Channel3 │ │ Channel4 │       │
│  │  Input   │ │  Output  │ │   MCP    │ │ PageLLM  │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 多进程 IPC 架构

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│   Firefox Extension                                          │
│     │                                                        │
│     ├─ Port1 ──> nevoflux-agent --channel=input ────┐       │
│     │                                                │       │
│     ├─ Port2 ──> nevoflux-agent --channel=output ───┤       │
│     │                                                │       │
│     ├─ Port3 ──> nevoflux-agent --channel=mcp ──────┤       │
│     │                                                │       │
│     └─ Port4 ──> nevoflux-agent --channel=pagellm ──┤       │
│                                                      │       │
│                                                      ▼       │
│                              ┌─────────────────────────┐     │
│                              │   AgentCore (守护进程)   │     │
│                              │   /tmp/nevoflux.sock    │     │
│                              └─────────────────────────┘     │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 核心接口定义

### 3.1 AgentCore

```rust
// core/mod.rs
pub struct AgentCore {
    // 共享状态
    state: Arc<AgentState>,

    // 功能管理器
    session_mgr: Arc<SessionManager>,
    llm_mgr: Arc<LlmManager>,
    tool_mgr: Arc<ToolManager>,
    mcp_mgr: Arc<McpManager>,
    skill_mgr: Arc<SkillManager>,
    perm_mgr: Arc<PermissionManager>,

    // 输出通道（用于向 Sidebar 发送消息）
    output_tx: mpsc::UnboundedSender<OutputMessage>,
}

impl AgentCore {
    /// 处理 Channel 1 输入消息
    pub async fn handle_input(&self, msg: InputMessage) -> Result<()>;

    /// 处理 Channel 3 MCP 请求
    pub async fn handle_mcp_request(&self, msg: McpRequestPayload) -> Result<McpResponsePayload>;

    /// 处理 Channel 4 Page LLM 请求
    pub async fn handle_page_llm_request(&self, msg: PageLlmRequestPayload) -> Result<()>;

    /// 发送输出消息到 Channel 2
    pub async fn send_output(&self, msg: OutputMessage) -> Result<()>;
}
```

### 3.2 LlmManager（4 种模式）

```rust
// managers/llm.rs
pub struct LlmManager {
    /// API 模式客户端 (Anthropic/OpenAI/Ollama)
    api_client: Option<LlmClient>,

    /// 本地模式 (Candle) - 未来实现
    local_client: Option<LocalLlmClient>,

    /// 账户订阅模式 - 未来实现
    subscription_client: Option<SubscriptionLlmClient>,

    /// Page LLM 通道发送器
    page_llm_tx: Option<mpsc::UnboundedSender<PageLlmMessage>>,
}

#[derive(Clone)]
pub enum LlmMode {
    Api,           // llama.cpp/Ollama/OpenAI/Anthropic
    Local,         // Candle
    Subscription,  // NevoFlux 账户
    PageMode,      // 通过 Extension Browser Use API
}

impl LlmManager {
    /// 流式聊天（根据当前模式自动选择）
    pub async fn chat_stream(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        mode: LlmMode,
    ) -> Result<impl Stream<Item = StreamChunkPayload>>;

    /// 带工具的聊天（Agentic Loop 核心）
    pub async fn chat_with_tools(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse>;
}
```

### 3.3 PermissionManager（Human in the Loop）

```rust
// managers/permission.rs
pub struct PermissionManager {
    /// 输出通道
    output_tx: mpsc::UnboundedSender<OutputMessage>,

    /// 等待中的请求 (request_id -> oneshot sender)
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<PermissionResponsePayload>>>>,

    /// 权限缓存 (resource_key -> PermissionScope)
    cache: Arc<RwLock<PermissionCache>>,
}

impl PermissionManager {
    /// 请求权限（阻塞直到用户响应或超时）
    pub async fn request_permission(
        &self,
        resource_type: ResourceType,
        action: ResourceAction,
        resource: &str,
        requester: Requester,
        reason: &str,
    ) -> Result<bool, PermissionError>;

    /// 处理用户的权限响应
    pub async fn handle_response(&self, response: PermissionResponsePayload) -> Result<()>;

    /// 检查是否已有授权（不阻塞）
    pub fn check_cached(&self, resource_type: ResourceType, action: ResourceAction, resource: &str) -> Option<bool>;

    /// 清除会话级别的权限缓存
    pub async fn clear_session_cache(&self);
}

#[derive(Debug)]
pub enum PermissionError {
    Denied,
    Timeout,
    ChannelClosed,
}
```

### 3.4 IPC 协议

```rust
// core/ipc.rs

/// IPC 消息包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    /// 消息来源通道
    pub channel: ChannelType,
    /// 消息内容（JSON 字符串）
    pub payload: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChannelType {
    Input,    // Channel 1
    Output,   // Channel 2
    Mcp,      // Channel 3
    PageLlm,  // Channel 4
}

/// IPC 传输格式：4字节长度前缀 + JSON
/// (与 Native Messaging 协议一致)
```

---

## 4. 文件结构

### 4.1 新建文件

| 文件路径 | 说明 |
|---------|------|
| **Rust Agent** | |
| `crates/nevoflux-agent/src/core/mod.rs` | AgentCore 主结构 |
| `crates/nevoflux-agent/src/core/router.rs` | 消息路由器 |
| `crates/nevoflux-agent/src/core/state.rs` | 共享状态 |
| `crates/nevoflux-agent/src/core/daemon.rs` | 守护进程逻辑 |
| `crates/nevoflux-agent/src/core/ipc.rs` | IPC 协议定义 |
| `crates/nevoflux-agent/src/managers/mod.rs` | 管理器模块导出 |
| `crates/nevoflux-agent/src/managers/llm.rs` | LLM 调度（4种模式） |
| `crates/nevoflux-agent/src/managers/tool.rs` | 工具执行管理 |
| `crates/nevoflux-agent/src/managers/mcp.rs` | MCP Client/Server |
| `crates/nevoflux-agent/src/managers/skill.rs` | Skills 管理 |
| `crates/nevoflux-agent/src/managers/permission.rs` | 权限管理 |
| `crates/nevoflux-agent/src/native_messaging/proxy.rs` | 通道代理 |
| **Extension** | |
| `extensions/.../native/com.nevoflux.agent.input.json` | Input 通道 manifest |
| `extensions/.../native/com.nevoflux.agent.output.json` | Output 通道 manifest |
| `extensions/.../native/com.nevoflux.agent.mcp.json` | MCP 通道 manifest |
| `extensions/.../native/com.nevoflux.agent.pagellm.json` | PageLLM 通道 manifest |
| `extensions/.../background/channel-manager.js` | 多通道管理器 |

### 4.2 修改文件

| 文件路径 | 修改内容 |
|---------|---------|
| **Rust Agent** | |
| `crates/nevoflux-agent/src/main.rs` | 多模式启动逻辑 |
| `crates/nevoflux-agent/src/channels/mod.rs` | 导出 + 接入 AgentCore |
| `crates/nevoflux-agent/src/channels/input.rs` | 接入 AgentCore |
| `crates/nevoflux-agent/src/channels/output.rs` | 接入 AgentCore |
| `crates/nevoflux-agent/src/channels/mcp.rs` | 接入 McpManager |
| `crates/nevoflux-agent/src/channels/page_llm.rs` | 接入 LlmManager |
| `crates/nevoflux-agent/src/native_messaging/mod.rs` | 多端口支持 |
| `crates/nevoflux-agent/Cargo.toml` | 添加 IPC 依赖 |
| **Extension** | |
| `extensions/.../background/background.js` | 使用 ChannelManager |
| `extensions/.../manifest.json` | 添加新 native messaging 权限 |

### 4.3 删除/废弃文件

| 文件路径 | 说明 |
|---------|------|
| `crates/nevoflux-agent/src/native_messaging/handler.rs` | 旧逻辑移入 core/ |
| `crates/nevoflux-agent/src/session.rs` | 移入 managers/ |
| `crates/nevoflux-agent/src/stream_manager.rs` | 移入 managers/ |
| `crates/nevoflux-agent/src/action_router.rs` | 移入 core/router.rs |

---

## 5. 实现计划

### Week 1: 基础架构

| Step | 任务 | 说明 |
|------|------|------|
| 1 | 创建 core/ 模块骨架 | AgentCore, router, state |
| 2 | 创建 managers/ 模块骨架 | 各管理器接口定义 |
| 3 | 实现 IPC 协议 | ipc.rs, Unix Socket 通信 |
| 4 | 修改 main.rs | 支持 --daemon 和 --channel 参数 |

### Week 2: 通道集成

| Step | 任务 | 说明 |
|------|------|------|
| 5 | 实现 daemon.rs | AgentCore 守护进程 |
| 6 | 实现 proxy.rs | 通道代理进程 |
| 7 | 修改 channels/ | 接入 AgentCore |
| 8 | 创建 Extension manifests | 4 个 native messaging manifest |

### Week 3: 功能管理器

| Step | 任务 | 说明 |
|------|------|------|
| 9 | 实现 PermissionManager | Human in the Loop |
| 10 | 完善 LlmManager | 整合现有代码，支持 4 种模式 |
| 11 | 完善 ToolManager | 工具执行和结果处理 |
| 12 | 集成测试 | 端到端测试 |

---

## 6. 权限请求流程

```
┌─────────────────────────────────────────────────────────────┐
│                    权限请求流程                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. ToolManager 需要读取文件                                 │
│     │                                                       │
│     ▼                                                       │
│  2. PermissionManager.request_permission(...)               │
│     │                                                       │
│     ├─ 检查缓存：是否已有 Always/Session 授权？              │
│     │   ├─ 有 → 直接返回 granted                            │
│     │   └─ 无 → 继续                                        │
│     │                                                       │
│     ▼                                                       │
│  3. 发送 PermissionRequest 到 Channel 2                     │
│     │                                                       │
│     ▼                                                       │
│  4. 等待 Channel 1 的 PermissionResponse                    │
│     │  (使用 oneshot channel + timeout)                     │
│     │                                                       │
│     ▼                                                       │
│  5. 收到响应                                                 │
│     ├─ granted=true  → 缓存授权，返回 Ok                     │
│     ├─ granted=false → 返回 PermissionDenied                │
│     └─ timeout       → 返回 PermissionTimeout               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. Extension 端配置

### 7.1 Native Messaging Manifests

```json
// com.nevoflux.agent.input.json
{
  "name": "com.nevoflux.agent.input",
  "description": "NevoFlux Agent - Input Channel",
  "path": "/usr/local/bin/nevoflux-agent",
  "type": "stdio",
  "allowed_extensions": ["agent@nevoflux.com"]
}
```

### 7.2 ChannelManager

```javascript
// background/channel-manager.js
class ChannelManager {
  constructor() {
    this.ports = {
      input: null,
      output: null,
      mcp: null,
      pagellm: null,
    };
  }

  async connectAll() {
    this.ports.input = await this.connect('com.nevoflux.agent.input');
    this.ports.output = await this.connect('com.nevoflux.agent.output');

    this.ports.output.onMessage.addListener((msg) => {
      this.handleOutputMessage(msg);
    });

    if (await this.isMcpEnabled()) {
      this.ports.mcp = await this.connect('com.nevoflux.agent.mcp');
    }
  }

  sendInput(message) {
    if (this.ports.input) {
      this.ports.input.postMessage(message);
    }
  }
}
```

---

## 8. 版本历史

| 版本 | 日期 | 说明 |
|-----|------|------|
| 1.0.0 | 2026-01-18 | 初始设计完成 |
