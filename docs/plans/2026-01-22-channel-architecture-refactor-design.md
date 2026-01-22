# 通道架构重构设计 (4通道 → 2通道)

> 版本: 5.0.0
> 日期: 2026-01-22
> 状态: 设计完成

---

## 1. 架构概述

### 1.1 变更前（4通道）

```
┌─────────────┐          ┌─────────────┐
│   Sidebar   │          │Native Agent │
└──────┬──────┘          └──────┬──────┘
       │                        │
       │ Input ────────────────►│
       │◄──────────────── Output│
       │◄────────────────► MCP  │
       │◄───────────── PageLLM  │ (删除)
       │                        │
```

### 1.2 变更后（2通道）

```
┌─────────────┐          ┌─────────────┐
│   Sidebar   │          │Native Agent │
└──────┬──────┘          └──────┬──────┘
       │                        │
       │◄────────────────► Chat │  com.nevoflux.agent
       │◄────────────────► MCP  │  com.nevoflux.agent.mcp
       │                        │
```

### 1.3 核心变更

| 项目 | 变更前 | 变更后 |
|-----|--------|--------|
| 通道数量 | 4 | 2 |
| Input/Output | 分离 | 合并为 Chat |
| PageLLM | 存在 | 删除 |
| MCP | 保留 | 保留 |
| background.js 角色 | 路由 + 执行 | 纯通道 + API 提供者 |
| Browser Tool 决策 | background.js | Sidebar |
| 协议版本 | 4.0.0 | 5.0.0 |

### 1.4 设计原则

- **background.js 作为纯通道**：只负责消息传递和提供 API，不做业务决策
- **Sidebar 作为控制者**：所有 Browser Tool 执行由 Sidebar 决定
- **Native Agent 负责权限**：权限控制在 Native Agent 中，Sidebar 信任其请求
- **API 命名空间**：background.js API 使用 `bg:` 前缀，避免消息路由混乱

---

## 2. shared-protocol Crate 重构

### 2.1 文件结构变更

```
src/nevoflux/extensions/nevoflux-agent/dioxus-ui/shared-protocol/src/
├── lib.rs           # 导出所有类型
├── chat.rs          # Chat 通道消息（新，合并 channel1 + channel2）
├── mcp.rs           # MCP 通道消息（重命名自 channel3）
├── common.rs        # 共享类型（保留）
├── channel1.rs      # 删除
├── channel2.rs      # 删除
├── channel3.rs      # 删除
├── channel4.rs      # 删除
```

### 2.2 chat.rs 消息类型

```rust
/// Chat 通道消息 - 双向通信
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ChatMessage {
    // ========== Sidebar → Agent ==========
    /// 用户聊天消息
    ChatMessage(ChatMessagePayload),
    /// 触发 Skill
    SkillCommand(SkillCommandPayload),
    /// 停止生成
    StopGeneration(StopGenerationPayload),
    /// 用户授权响应
    PermissionResponse(PermissionResponsePayload),
    /// 插件指令
    PluginCommand(PluginCommandPayload),
    /// 系统指令
    SystemCommand(SystemCommandPayload),
    /// Browser Tool 响应（Sidebar → Agent）
    BrowserToolResponse(BrowserToolResponsePayload),

    // ========== Agent → Sidebar ==========
    /// 流式文本响应
    StreamChunk(StreamChunkPayload),
    /// 流结束
    StreamEnd(StreamEndPayload),
    /// 完整内容块
    ContentBlock(ContentBlockPayload),
    /// 请求用户授权
    PermissionRequest(PermissionRequestPayload),
    /// Agent 状态更新
    AgentState(AgentStatePayload),
    /// 错误通知
    Error(ErrorPayload),
    /// 账户状态
    AccountStatus(AccountStatusPayload),
    /// 系统指令响应
    SystemResponse(SystemResponsePayload),
    /// Browser Tool 请求（Agent → Sidebar）
    BrowserToolRequest(BrowserToolRequestPayload),
}
```

### 2.3 辅助方法

```rust
impl ChatMessage {
    /// 判断消息方向
    pub fn direction(&self) -> MessageDirection {
        match self {
            // Sidebar → Agent
            ChatMessage::ChatMessage(_) |
            ChatMessage::SkillCommand(_) |
            ChatMessage::StopGeneration(_) |
            ChatMessage::PermissionResponse(_) |
            ChatMessage::PluginCommand(_) |
            ChatMessage::SystemCommand(_) |
            ChatMessage::BrowserToolResponse(_) => MessageDirection::ToAgent,

            // Agent → Sidebar
            _ => MessageDirection::ToSidebar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    ToAgent,
    ToSidebar,
}
```

---

## 3. background.js 重构

### 3.1 API 命名空间

```javascript
// Background API（Sidebar 调用的接口，"bg:" 前缀）
const BackgroundAPI = {
  // 通道管理
  CONNECT: "bg:connect",              // 连接 Chat 通道
  DISCONNECT: "bg:disconnect",        // 断开连接
  GET_STATUS: "bg:get_status",        // 获取连接状态

  // MCP 通道管理
  MCP_ENABLE: "bg:mcp_enable",        // 启用 MCP 通道
  MCP_DISABLE: "bg:mcp_disable",      // 禁用 MCP 通道

  // 消息发送
  SEND_TO_AGENT: "bg:send_to_agent",  // 发送消息给 Native Agent

  // Browser Tool 执行
  EXEC_TOOL: "bg:exec_tool",          // 执行 browser tool

  // Tab 上下文
  GET_TAB_CONTEXT: "bg:get_tab_context",  // 获取当前 tab 信息
};
```

### 3.2 通道名称

```javascript
const CHANNEL_NAMES = {
  CHAT: "com.nevoflux.agent",      // Chat 通道（双向）
  MCP: "com.nevoflux.agent.mcp",   // MCP 通道（双向）
};
```

### 3.3 ChannelManager 简化

```javascript
class ChannelManager {
  constructor() {
    // Chat 通道：Sidebar ↔ Agent
    this.chat = new NativeChannel(
      CHANNEL_NAMES.CHAT,
      "Chat",
      (msg) => this.handleChatMessage(msg),
      (connected, error) => this.handleChatStatusChange(connected, error)
    );

    // MCP 通道：Browser Use MCP
    this.mcp = new NativeChannel(
      CHANNEL_NAMES.MCP,
      "MCP",
      (msg) => this.handleMcpMessage(msg),
      (connected, error) => this.handleMcpStatusChange(connected, error)
    );

    this.connectionStatus = { chat: false, mcp: false };
    this.mcpEnabled = false;
  }

  connect() {
    this.chat.connect();
  }

  disconnect() {
    this.chat.disconnect();
    if (this.mcpEnabled) {
      this.mcp.disconnect();
    }
  }

  sendToAgent(message) {
    return this.chat.send(message);
  }

  handleChatMessage(message) {
    // 检查是否为分块消息
    if (chunkReassembler.isChunk(message)) {
      const reassembled = chunkReassembler.processChunk(message);
      if (reassembled) {
        broadcastToSidebar(reassembled);
      }
      return;
    }

    // 所有消息都广播给 Sidebar，由 Sidebar 决定处理
    broadcastToSidebar(message);
  }

  getStatus() {
    return {
      connected: this.connectionStatus.chat,
      channels: { ...this.connectionStatus },
    };
  }
}
```

### 3.4 消息监听器

```javascript
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
  const msgType = message.type;

  // Background API 处理（"bg:" 前缀）
  if (msgType.startsWith("bg:")) {
    return handleBackgroundAPI(msgType, message, sendResponse);
  }

  // 其他消息忽略（由 Sidebar 处理）
});

function handleBackgroundAPI(apiType, message, sendResponse) {
  switch (apiType) {
    case BackgroundAPI.CONNECT:
      channelManager.connect();
      sendResponse({ success: true });
      break;

    case BackgroundAPI.DISCONNECT:
      channelManager.disconnect();
      sendResponse({ success: true });
      break;

    case BackgroundAPI.GET_STATUS:
      sendResponse(channelManager.getStatus());
      break;

    case BackgroundAPI.MCP_ENABLE:
      channelManager.setMcpEnabled(true);
      sendResponse({ success: true });
      break;

    case BackgroundAPI.MCP_DISABLE:
      channelManager.setMcpEnabled(false);
      sendResponse({ success: true });
      break;

    case BackgroundAPI.SEND_TO_AGENT:
      const sent = channelManager.sendToAgent(message.payload);
      sendResponse({ success: sent });
      break;

    case BackgroundAPI.EXEC_TOOL:
      executeBrowserTool(message.payload)
        .then(result => sendResponse(result))
        .catch(err => sendResponse({
          success: false,
          error: { message: err.message }
        }));
      return true; // 保持 sendResponse 有效

    case BackgroundAPI.GET_TAB_CONTEXT:
      getActiveTabContext()
        .then(ctx => sendResponse(ctx))
        .catch(err => sendResponse(null));
      return true;

    default:
      sendResponse({ success: false, error: "Unknown API" });
  }
}
```

---

## 4. 消息流程

### 4.1 用户发送聊天消息

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Sidebar   │         │ background.js│         │Native Agent │
└──────┬──────┘         └──────┬───────┘         └──────┬──────┘
       │                       │                        │
       │ bg:send_to_agent      │                        │
       │ {type:"chat_message"} │                        │
       │──────────────────────►│                        │
       │                       │ chat_message           │
       │                       │───────────────────────►│
       │                       │                        │
       │                       │      stream_chunk      │
       │    stream_chunk       │◄───────────────────────│
       │◄──────────────────────│                        │
       │                       │      stream_end        │
       │    stream_end         │◄───────────────────────│
       │◄──────────────────────│                        │
       │                       │                        │
```

### 4.2 Browser Tool 执行流程

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Sidebar   │         │ background.js│         │Native Agent │
└──────┬──────┘         └──────┬───────┘         └──────┬──────┘
       │                       │                        │
       │                       │  browser_tool_request  │
       │ browser_tool_request  │◄───────────────────────│
       │◄──────────────────────│                        │
       │                       │                        │
       │ (显示执行状态)         │                        │
       │                       │                        │
       │ bg:exec_tool          │                        │
       │ {action:"snapshot"..} │                        │
       │──────────────────────►│                        │
       │                       │                        │
       │   (执行 browser API)   │                        │
       │                       │                        │
       │ {success:true,result} │                        │
       │◄──────────────────────│                        │
       │                       │                        │
       │ (更新 UI，准备响应)    │                        │
       │                       │                        │
       │ bg:send_to_agent      │                        │
       │ {type:"browser_tool_  │                        │
       │  response", ...}      │                        │
       │──────────────────────►│                        │
       │                       │ browser_tool_response  │
       │                       │───────────────────────►│
       │                       │                        │
```

### 4.3 权限请求流程

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Sidebar   │         │ background.js│         │Native Agent │
└──────┬──────┘         └──────┬───────┘         └──────┬──────┘
       │                       │                        │
       │                       │  permission_request    │
       │ permission_request    │◄───────────────────────│
       │◄──────────────────────│                        │
       │                       │                        │
       │ (显示授权对话框)       │                        │
       │                       │                        │
       │ 用户点击"允许"         │                        │
       │                       │                        │
       │ bg:send_to_agent      │                        │
       │ {type:"permission_    │                        │
       │  response", ...}      │                        │
       │──────────────────────►│                        │
       │                       │ permission_response    │
       │                       │───────────────────────►│
       │                       │                        │
```

---

## 5. 大消息分块机制

### 5.1 分块配置

```javascript
const CHUNK_CONFIG = {
  maxMessageSize: 900_000,  // 900KB 阈值（留 100KB 缓冲）
  chunkSize: 800_000,       // 每块 800KB
  timeout: 30_000,          // 重组超时 30 秒
};
```

### 5.2 分块处理流程

```
发送方向 (Sidebar → Agent):
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Sidebar   │         │ background.js│         │Native Agent │
└──────┬──────┘         └──────┬───────┘         └──────┬──────┘
       │                       │                        │
       │ bg:send_to_agent      │                        │
       │ (大消息)               │                        │
       │──────────────────────►│                        │
       │                       │ [自动分块]              │
       │                       │ chunk 1/3 ───────────►│
       │                       │ chunk 2/3 ───────────►│
       │                       │ chunk 3/3 ───────────►│
       │                       │                        │ [重组]
       │                       │                        │

接收方向 (Agent → Sidebar):
       │                       │                        │
       │                       │ chunk 1/3 ◄───────────│
       │                       │ chunk 2/3 ◄───────────│
       │                       │ chunk 3/3 ◄───────────│
       │                       │ [重组]                 │
       │ (完整消息)             │                        │
       │◄──────────────────────│                        │
```

### 5.3 NativeChannel.send() 方法

```javascript
class NativeChannel {
  /**
   * 发送消息，自动处理大消息分块
   * Automatically chunks large messages to handle Firefox's 1MB native messaging limit
   */
  send(message) {
    if (!this.port) {
      console.warn(`[NevoFlux] Cannot send to ${this.displayName} - not connected`);
      return false;
    }

    try {
      if (needsChunking(message)) {
        const chunks = chunkMessage(message);
        for (const chunk of chunks) {
          this.port.postMessage(chunk);
        }
        console.log(`[NevoFlux] ${this.displayName} sent ${chunks.length} chunks`);
        return true;
      }

      this.port.postMessage(message);
      return true;
    } catch (error) {
      console.error(`[NevoFlux] Failed to send to ${this.displayName}:`, error);
      return false;
    }
  }
}
```

### 5.4 ChunkReassembler

```javascript
class ChunkReassembler {
  constructor() {
    // Map: chunkId -> { chunks: Map<index, data>, total, timestamp }
    this.pending = new Map();
  }

  isChunk(message) {
    return message?.__chunk != null;
  }

  processChunk(chunkEnvelope) {
    const { id, index, total, data } = chunkEnvelope.__chunk;

    let pending = this.pending.get(id);
    if (!pending) {
      pending = { chunks: new Map(), total, timestamp: Date.now() };
      this.pending.set(id, pending);
    }

    pending.chunks.set(index, data);

    if (pending.chunks.size === total) {
      let fullBase64 = "";
      for (let i = 0; i < total; i++) {
        fullBase64 += pending.chunks.get(i);
      }

      try {
        const json = decodeURIComponent(escape(atob(fullBase64)));
        const message = JSON.parse(json);
        this.pending.delete(id);
        this.cleanupOldPending();
        return message;
      } catch (e) {
        console.error(`[NevoFlux] Failed to reassemble message ${id}:`, e);
        this.pending.delete(id);
        return null;
      }
    }

    return null;
  }

  cleanupOldPending() {
    const now = Date.now();
    for (const [id, pending] of this.pending) {
      if (now - pending.timestamp > CHUNK_CONFIG.timeout) {
        console.warn(`[NevoFlux] Chunk reassembly timed out for message ${id}`);
        this.pending.delete(id);
      }
    }
  }
}
```

---

## 6. 文件变更清单

### 6.1 shared-protocol Crate

| 文件 | 操作 | 说明 |
|-----|------|------|
| `lib.rs` | 修改 | 更新导出，移除 channel1-4，添加 chat/mcp |
| `chat.rs` | 新建 | 合并 channel1 + channel2 的消息类型 |
| `mcp.rs` | 重命名 | 从 channel3.rs 重命名 |
| `common.rs` | 保留 | 共享类型不变 |
| `channel1.rs` | 删除 | — |
| `channel2.rs` | 删除 | — |
| `channel3.rs` | 删除 | 内容移至 mcp.rs |
| `channel4.rs` | 删除 | — |

### 6.2 Extension

| 文件 | 操作 | 说明 |
|-----|------|------|
| `background/background.js` | 重构 | API 命名空间、通道简化 |

### 6.3 Native Agent (Rust)

| 文件 | 操作 | 说明 |
|-----|------|------|
| `nevoflux-agent/src/channels/` | 修改 | 适配新的 2 通道架构 |
| `nevoflux-agent/src/main.rs` | 修改 | 移除 PageLLM 通道处理 |

### 6.4 Sidebar (Dioxus)

| 文件 | 操作 | 说明 |
|-----|------|------|
| `messaging/*.rs` | 修改 | 使用新的 BackgroundAPI 调用方式 |

---

## 7. 迁移注意事项

### 7.1 向后兼容

- Native Agent 需要同步更新，不保持向后兼容
- 协议版本从 `4.0.0` 升级到 `5.0.0`

### 7.2 测试重点

1. Chat 通道双向通信
2. Browser Tool 执行流程（Sidebar 控制）
3. 权限请求/响应流程
4. MCP 通道独立工作
5. 连接断开/重连
6. 大消息分块/重组

### 7.3 删除的功能

- PageLLM 通道及相关代码
- `SINGLE_CHANNEL_MODE` 常量（现在只有单通道模式）
- Output 通道相关代码
- `channel1.rs`, `channel2.rs`, `channel3.rs`, `channel4.rs`

---

## 8. 版本历史

| 版本 | 日期 | 说明 |
|-----|------|------|
| 5.0.0 | 2026-01-22 | 4通道 → 2通道架构重构 |
| 4.0.0 | 2026-01-18 | 4通道架构初始设计 |
