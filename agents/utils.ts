import { $ } from "bun";
import { existsSync } from "fs";
import { readdir } from "fs/promises";
import path from "path";

export type AgentState = {
  agents: Agent[];
};

export enum AgentStatus {
  Idle,
  Working,
  Stalled,
}

export type Agent = {
  name: string;
  sessionId: string;
  status: AgentStatus;
  lastActivity: string; // ISO timestamp
  currentTask: string; // Task description - persists after completion, overwritten on new task
};

export type ClaudeHook = {
  // Common fields
  session_id: String;
  transcript_path: String; // Path to conversation JSON
  cwd: String; // The current working directory when the hook is invoked

  // Event-specific fields
  hook_event_name: String;
};

// File locking system types
export type FileLock = {
  sessionId: string;
  agentName: string | null;
  lockTime: string; // ISO timestamp
  operation: string; // Description of the operation
  filePath: string;
};

export type FileLockState = {
  locks: Record<string, FileLock>; // Key is the absolute file path
  blockedAgents: Array<{
    agentName: string;
    sessionId: string;
    requestedFile: string;
    requestTime: string;
  }>;
};

const SCRIPT_DIR = import.meta.dir;
export const AGENT_STATE_FILE = path.join(SCRIPT_DIR, "agent_state.json");
export const FILE_LOCKS_FILE = path.join(SCRIPT_DIR, "file_locks.json");

export async function loadAgentState(): Promise<AgentState> {
  if (!existsSync(AGENT_STATE_FILE)) {
    const initialState: AgentState = { agents: [] };
    await Bun.write(AGENT_STATE_FILE, JSON.stringify(initialState, null, 2));
    return initialState;
  }

  const content = await Bun.file(AGENT_STATE_FILE).text();
  return JSON.parse(content) as AgentState;
}

export async function saveAgentState(state: AgentState): Promise<void> {
  await Bun.write(AGENT_STATE_FILE, JSON.stringify(state, null, 2));
}

export async function sendToAgent(
  windowName: string,
  command: string,
): Promise<void> {
  await $`tmux send-keys -t ${windowName} -l ${command}`;
  await new Promise((resolve) => setTimeout(resolve, 500));
  await $`tmux send-keys -t ${windowName} C-Enter`;
}

// Claude Code JSONL message types
export type ClaudeMessageBase = {
  parentUuid: string | null;
  isSidechain: boolean;
  userType: string;
  cwd: string;
  sessionId: string;
  version: string;
  uuid: string;
  timestamp: string;
};

export type TextContent = {
  type: "text";
  text: string;
};

export type ToolUseContent = {
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, any>;
};

export type ToolResultContent = {
  type: "tool_result";
  tool_use_id: string;
  content: string | any;
};

export type MessageContent = TextContent | ToolUseContent | ToolResultContent;

// Discriminated union for different message types
export type UserMessage = ClaudeMessageBase & {
  type: "user";
  message: {
    role: "user";
    content: string | MessageContent[];
  };
  toolUseResult?: any; // Present for tool results
};

export type AssistantMessage = ClaudeMessageBase & {
  type: "assistant";
  message: {
    id: string;
    type: "message";
    role: "assistant";
    model: string;
    content: MessageContent[];
    stop_reason: string | null;
    stop_sequence: string | null;
    usage: {
      input_tokens: number;
      cache_creation_input_tokens?: number;
      cache_read_input_tokens?: number;
      output_tokens: number;
      service_tier: string;
    };
  };
  requestId: string;
};

export type ClaudeMessage = UserMessage | AssistantMessage;

// Processed message types for display
export type ProcessedUserMessage = {
  messageType: "user";
  content: string;
  timestamp: string;
  index: number;
};

export type ProcessedToolResultMessage = {
  messageType: "tool_result";
  content: string;
  timestamp: string;
  index: number;
  toolUseId: string;
};

export type ProcessedAssistantMessage = {
  messageType: "assistant";
  content: string;
  timestamp: string;
  index: number;
};

export type ProcessedToolUseMessage = {
  messageType: "tool_use";
  toolName: string;
  arguments: string;
  timestamp: string;
  index: number;
};

export type ProcessedMessage = 
  | ProcessedUserMessage 
  | ProcessedToolResultMessage 
  | ProcessedAssistantMessage 
  | ProcessedToolUseMessage;

export type ParsedConversation = {
  messages: ProcessedMessage[];
  totalMessages: number;
};

function parseClaudeMessage(jsonObj: any): ClaudeMessage | null {
  // Type guard for user messages
  if (jsonObj.type === "user" && jsonObj.message?.role === "user") {
    return jsonObj as UserMessage;
  }
  
  // Type guard for assistant messages
  if (jsonObj.type === "assistant" && jsonObj.message?.role === "assistant") {
    return jsonObj as AssistantMessage;
  }
  
  return null;
}

function processMessage(claudeMessage: ClaudeMessage, index: number): ProcessedMessage[] {
  const timestamp = claudeMessage.timestamp;
  const processed: ProcessedMessage[] = [];
  
  if (claudeMessage.type === "user") {
    const content = claudeMessage.message.content;
    
    // Handle array content (tool results)
    if (Array.isArray(content)) {
      // Check if this is a tool result
      const toolResult = content.find((item: any) => item.type === "tool_result");
      if (toolResult) {
        processed.push({
          messageType: "tool_result",
          content: typeof toolResult.content === 'string' ? toolResult.content : JSON.stringify(toolResult.content),
          timestamp,
          index,
          toolUseId: toolResult.tool_use_id
        });
      } else {
        // Regular user message with structured content
        const textContent = content
          .filter((item: any) => item.type === "text")
          .map((item: any) => item.text)
          .join(" ");
        
        if (textContent.trim()) {
          processed.push({
            messageType: "user",
            content: textContent,
            timestamp,
            index
          });
        }
      }
    } else if (typeof content === "string" && content.trim()) {
      // Handle string content (simple user messages)
      processed.push({
        messageType: "user",
        content: content,
        timestamp,
        index
      });
    }
  } else if (claudeMessage.type === "assistant") {
    const content = claudeMessage.message.content;
    
    for (const item of content) {
      if (item.type === "text" && item.text && item.text.trim()) {
        processed.push({
          messageType: "assistant",
          content: item.text,
          timestamp,
          index
        });
      } else if (item.type === "tool_use" && item.name) {
        const args = Object.entries(item.input || {})
          .map(([key, value]) => `${key}: ${JSON.stringify(value)}`)
          .join(", ");
        
        processed.push({
          messageType: "tool_use",
          toolName: item.name,
          arguments: args,
          timestamp,
          index
        });
      }
    }
  }
  
  return processed;
}

export async function parseConversationFile(filePath: string): Promise<ParsedConversation> {
  if (!existsSync(filePath)) {
    throw new Error(`Conversation file not found: ${filePath}`);
  }
  
  const content = await Bun.file(filePath).text();
  
  // Parse JSONL format (each line is a separate JSON object)
  const lines = content.trim().split('\n').filter(line => line.trim());
  const processedMessages: ProcessedMessage[] = [];
  let messageIndex = 0;
  
  for (const line of lines) {
    try {
      const jsonObj = JSON.parse(line);
      const claudeMessage = parseClaudeMessage(jsonObj);
      
      if (claudeMessage) {
        const processed = processMessage(claudeMessage, messageIndex);
        processedMessages.push(...processed);
        messageIndex++;
      }
    } catch (parseError) {
      // Skip invalid JSON lines
      continue;
    }
  }
  
  return {
    messages: processedMessages,
    totalMessages: processedMessages.length
  };
}

function indentMultilineContent(content: string): string {
  const lines = content.split('\n');
  if (lines.length <= 1) {
    return content;
  }
  
  // First line stays as-is, subsequent lines get 4-space indent
  return lines[0] + '\n' + lines.slice(1).map(line => '    ' + line).join('\n');
}

export function formatProcessedMessage(message: ProcessedMessage): string {
  const timestamp = new Date(message.timestamp).toISOString();
  
  // Use discriminated union for type-safe formatting
  switch (message.messageType) {
    case "user": {
      let content = message.content || "";
      if (content.length > 2000) {
        content = content.substring(0, 2000) + "...";
      }
      const formattedContent = indentMultilineContent(content);
      return `[${message.index + 1}] ${timestamp} | user: ${formattedContent}`;
    }
      
    case "assistant": {
      let content = message.content || "";
      if (content.length > 2000) {
        content = content.substring(0, 2000) + "...";
      }
      const formattedContent = indentMultilineContent(content);
      return `[${message.index + 1}] ${timestamp} | assistant: ${formattedContent}`;
    }
      
    case "tool_use":
      return `[${message.index + 1}] ${timestamp} | ${message.toolName}(${message.arguments})`;
      
    case "tool_result": {
      let content = message.content || "";
      if (content.length > 2000) {
        content = content.substring(0, 2000) + "...";
      }
      const formattedContent = indentMultilineContent(content);
      return `[${message.index + 1}] ${timestamp} | [TOOL RESULT] ${formattedContent}`;
    }
      
    default:
      // This should never happen with proper discriminated unions
      const exhaustiveCheck: never = message;
      return `[${message.index + 1}] ${timestamp} | unknown: ${JSON.stringify(exhaustiveCheck)}`;
  }
}

export async function findConversationFile(sessionId: string): Promise<string | null> {
  // Claude stores conversation history in ~/.claude/projects/<project>/<session-id>.jsonl
  const claudeProjectsDir = path.join(process.env.HOME!, ".claude", "projects");
  
  if (!existsSync(claudeProjectsDir)) {
    return null;
  }
  
  try {
    // Read project directories using fs API
    const entries = await readdir(claudeProjectsDir, { withFileTypes: true });
    
    for (const entry of entries) {
      if (entry.isDirectory()) {
        const conversationFile = path.join(claudeProjectsDir, entry.name, `${sessionId}.jsonl`);
        
        if (existsSync(conversationFile)) {
          return conversationFile;
        }
      }
    }
  } catch (error) {
    console.error("Error searching for conversation file:", error);
  }
  
  return null;
}

// File locking utilities
export async function loadFileLockState(): Promise<FileLockState> {
  if (!existsSync(FILE_LOCKS_FILE)) {
    const initialState: FileLockState = { locks: {}, blockedAgents: [] };
    await Bun.write(FILE_LOCKS_FILE, JSON.stringify(initialState, null, 2));
    return initialState;
  }
  
  const content = await Bun.file(FILE_LOCKS_FILE).text();
  return JSON.parse(content) as FileLockState;
}

export async function saveFileLockState(state: FileLockState): Promise<void> {
  await Bun.write(FILE_LOCKS_FILE, JSON.stringify(state, null, 2));
}

export async function acquireFileLock(
  filePath: string, 
  sessionId: string, 
  operation: string,
  agentName?: string
): Promise<{ success: boolean; reason?: string }> {
  const absolutePath = path.resolve(filePath);
  const lockState = await loadFileLockState();
  
  // Check if file is already locked
  if (lockState.locks[absolutePath]) {
    const existingLock = lockState.locks[absolutePath];
    
    // If same session, allow (re-entrant lock)
    if (existingLock.sessionId === sessionId) {
      return { success: true };
    }
    
    // File is locked by another session
    return { 
      success: false, 
      reason: `File locked by session '${existingLock.sessionId}' since ${existingLock.lockTime}` 
    };
  }
  
  // Acquire the lock
  lockState.locks[absolutePath] = {
    sessionId,
    agentName: agentName || null,
    lockTime: new Date().toISOString(),
    operation,
    filePath: absolutePath
  };
  
  await saveFileLockState(lockState);
  return { success: true };
}

export async function releaseFileLock(filePath: string, sessionId: string): Promise<boolean> {
  const absolutePath = path.resolve(filePath);
  const lockState = await loadFileLockState();
  
  const existingLock = lockState.locks[absolutePath];
  if (!existingLock || existingLock.sessionId !== sessionId) {
    return false; // Lock doesn't exist or not owned by this session
  }
  
  // Remove the lock
  delete lockState.locks[absolutePath];
  
  await saveFileLockState(lockState);
  return true;
}

export async function releaseAllLocksForAgent(agentName: string): Promise<number> {
  const agentState = await loadAgentState();
  const agent = agentState.agents.find(a => a.name === agentName);
  
  if (!agent) {
    return 0; // Agent not found
  }
  
  return await releaseAllLocksForSession(agent.sessionId);
}

export async function releaseAllLocksForSession(sessionId: string): Promise<number> {
  const lockState = await loadFileLockState();
  let releasedCount = 0;
  
  // Find and release all locks held by this session
  for (const [filePath, lock] of Object.entries(lockState.locks)) {
    if (lock.sessionId === sessionId) {
      delete lockState.locks[filePath];
      releasedCount++;
    }
  }
  
  if (releasedCount > 0) {
    await saveFileLockState(lockState);
  }
  
  return releasedCount;
}

export async function checkFileLock(filePath: string): Promise<FileLock | null> {
  const absolutePath = path.resolve(filePath);
  const lockState = await loadFileLockState();
  return lockState.locks[absolutePath] || null;
}

export async function cleanupExpiredLocks(timeoutMinutes: number = 10): Promise<number> {
  const lockState = await loadFileLockState();
  const now = new Date();
  const timeoutMs = timeoutMinutes * 60 * 1000;
  let cleanedCount = 0;
  
  for (const [filePath, lock] of Object.entries(lockState.locks)) {
    const lockTime = new Date(lock.lockTime);
    if (now.getTime() - lockTime.getTime() > timeoutMs) {
      delete lockState.locks[filePath];
      cleanedCount++;
    }
  }
  
  if (cleanedCount > 0) {
    await saveFileLockState(lockState);
  }
  
  return cleanedCount;
}

/**
 * Determines agent name from agent_state.json by session ID
 */
export async function getAgentName(sessionId: string): Promise<string> {
  const agentState = await loadAgentState();
  const agent = agentState.agents.find((a) => a.sessionId === sessionId);

  if (agent) {
    return agent.name;
  } else {
    throw Error(`Cannot find an agent associated with sessionId ${sessionId}`);
  }
}

// Tool names that require file locking
export const FILE_WRITING_TOOLS = new Set([
  "Edit",
  "MultiEdit",
  "Write",
  "NotebookEdit",
]);

export const FILE_READING_TOOLS = new Set(["Read", "NotebookRead"]);

/**
 * Extracts file paths from tool arguments based on tool type
 */
export function extractFilePathsFromTool(toolName: string, toolArgs: any): string[] {
  const filePaths: string[] = [];

  switch (toolName) {
    case "Edit":
    case "Write":
    case "Read":
    case "NotebookEdit":
    case "NotebookRead":
      if (toolArgs.file_path || toolArgs.notebook_path) {
        filePaths.push(toolArgs.file_path || toolArgs.notebook_path);
      }
      break;

    case "MultiEdit":
      if (toolArgs.file_path) {
        filePaths.push(toolArgs.file_path);
      }
      break;
  }

  return filePaths.map((fp) => path.resolve(fp));
}
