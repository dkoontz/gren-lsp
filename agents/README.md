# Agent Orchestration System

A comprehensive system for managing multiple Claude Code agents with task coordination, file locking, and monitoring capabilities.

## 🏗️ System Architecture

The agent orchestration system consists of several interconnected components:

### Core Components

1. **Agent Management** - Create, monitor, and terminate Claude agents
2. **State Tracking** - Persistent state management for agents and their activities  
3. **File Locking** - Automatic file conflict prevention via hook-based sentinel files
4. **Hook Integration** - Pre/post tool hooks for automatic file locking and task completion detection
5. **Monitoring & Recovery** - Watchdog process to detect stalled agents and clean up resources
6. **Orchestrator Communication** - Notification system to report agent status to orchestrator

## 📁 File Structure

```
agents/
├── README.md                    # This documentation
├── utils.ts                     # Shared types, utilities, and file locking helpers
│
├── createAgent.ts               # Create new Claude agents
├── closeAgent.ts                # Shutdown and cleanup agents  
├── setAgentStatus.ts            # Update agent status (Idle/Working/Stalled)
├── getAgentStatus.ts            # Query agent status
├── getAgentHistory.ts           # Retrieve agent conversation history
├── sendToAgent.ts               # Send commands to agent tmux windows
│
├── onAgentComplete.ts           # Claude Code hook handler for task completion
├── preToolHook.ts               # Pre-tool hook for automatic file lock acquisition
├── postToolHook.ts              # Post-tool hook for automatic file lock release
├── setupHooks.ts                # Hook configuration helper
├── claude-hooks-config.json     # Claude Code hook configuration
│
├── fileLocking.ts               # Core file locking system with CLI interface
├── checkFileLock.ts             # Check if a file is locked
├── releaseFileLock.ts           # Manually release file locks
├── cleanupLocks.ts              # Clean up expired file locks
│
├── agentWatchdog.ts             # Monitor agents for stalls/crashes
├── notifyOrchestrator.ts        # Send notifications to orchestrator
│
├── agent_state.json             # Persistent agent state (auto-generated)
├── file_locks.json              # Legacy file locking state (deprecated)
└── .file-locks/                 # Directory containing lock sentinel files
```

## 🚀 Quick Start

### 1. Set Up Claude Code Hooks

```bash
# Get hook setup instructions
bun setupHooks.ts

# Follow the displayed instructions to configure Claude Code hooks
```

### 2. Create Your First Agent

```bash
# Create an agent named "dev"
bun createAgent.ts dev

# Check agent status
bun getAgentStatus.ts dev
```

### 3. Start Background Processes

```bash
# Start the watchdog to monitor for stalled agents
bun agentWatchdog.ts &

# Test file locking system
bun fileLocking.ts help
```

### 4. Assign Tasks to Agents

```bash
# Mark agent as working on a task
bun setAgentStatus.ts dev Working

# Send commands to the agent
bun sendToAgent.ts dev "Help me implement a new feature"

# Agent will automatically return to Idle when task completes (via hooks)
```

## 📚 Detailed Usage

### Agent Management

#### Creating Agents
```bash
bun createAgent.ts <agent-name>
```
- Creates a new tmux window with Claude Code
- Captures session ID for tracking
- Initializes agent state with Idle status

#### Querying Agent Status
```bash
bun getAgentStatus.ts <agent-name>
```
Returns: `Idle`, `Working`, or `Stalled`

#### Setting Agent Status
```bash
bun setAgentStatus.ts <agent-name> <status>
```
Valid statuses: `Idle`, `Working`, `Stalled`

#### Closing Agents
```bash
bun closeAgent.ts <agent-name>
```
- Sends `/exit` command
- Kills tmux window
- Removes from agent state
- Releases all file locks

### Communication

#### Sending Commands
```bash
bun sendToAgent.ts <agent-name> <command>
```
Sends text to the agent's tmux window.

#### Getting Conversation History
```bash
bun getAgentHistory.ts <agent-name> [line-count]
bun getAgentHistory.ts <session-id> [line-count]
```
Retrieves recent conversation messages (default: 50, max: 200).

### File Locking System

The file locking system automatically prevents conflicts when multiple agents work on the same files. It uses Claude Code hooks to acquire locks before tool execution and release them afterward.

#### Automatic Operation
- **Pre-tool hooks** automatically acquire locks for file operations (Edit, Write, Read, etc.)
- **Post-tool hooks** automatically release locks after operations complete
- **Timeout cleanup** removes expired locks (10-minute default timeout)

#### Manual File Lock Management
```bash
# Check if a file is locked
bun fileLocking.ts check <file-path>

# List all active locks
bun fileLocking.ts list

# Manually acquire a lock (for testing)
bun fileLocking.ts acquire <file-path> <agent-name> <session-id>

# Manually release a lock
bun fileLocking.ts release <file-path> <agent-name>

# Release all locks for an agent
bun fileLocking.ts release-agent <agent-name>

# Clean up expired locks
bun fileLocking.ts cleanup
```

#### Legacy Tools (still available)
```bash
bun checkFileLock.ts <file-path>
bun releaseFileLock.ts <file-path> <agent-name>
bun cleanupLocks.ts [timeout-minutes]
```

### Monitoring & Recovery

#### Agent Watchdog
```bash
# Default settings (30s checks, 5min stall timeout)
bun agentWatchdog.ts

# Custom settings  
bun agentWatchdog.ts --check-interval 60 --stall-timeout 10
```

The watchdog:
- Monitors tmux output for activity
- Detects stalled agents (no output change)
- Kills stalled agents and cleans up resources
- Notifies orchestrator of agent events

### Orchestrator Communication

#### Manual Notifications
```bash
bun notifyOrchestrator.ts <type> <message> [agent-name] [orchestrator-name]
```

Types: `agent_completed`, `agent_stalled`, `agent_crashed`, `file_lock_released`, `system_status`

## 🔧 Configuration

### Claude Code Hook Setup

The system uses three Claude Code hooks:
- **PreToolUse** - Acquires file locks before tool execution
- **PostToolUse** - Releases file locks after tool execution  
- **Stop** - Handles agent completion notifications

1. Locate your Claude Code settings file:
   - macOS/Linux: `~/.config/claude-code/settings.json`
   - Windows: `%APPDATA%\\claude-code\\settings.json`

2. Add the hook configuration from `claude-hooks-config.json`

3. Restart Claude Code

### Environment Variables

- `HOME` - Used to locate Claude conversation history
- Standard tmux and bun environment requirements

## 🏃‍♂️ Background Processes

For a fully functional system, run these background processes:

```bash
# Agent monitoring (recommended)
bun agentWatchdog.ts &

# Lock cleanup (periodic, via cron or similar - optional since hooks handle most cleanup)
*/5 * * * * cd /path/to/agents && bun fileLocking.ts cleanup
```

## 📊 State Files

### agent_state.json
```json
{
  "agents": [
    {
      "name": "dev",
      "sessionId": "abc123def456",
      "status": 0,
      "lastActivity": "2024-01-01T12:00:00.000Z",
      "currentTask": "Implement feature X"
    }
  ]
}
```

### .file-locks/ Directory
Individual lock files containing agent and lock information:
```bash
.file-locks/
├── Users-david-project-src-main.ts  # Lock sentinel for /Users/david/project/src/main.ts
└── path-to-another-file.js          # Lock sentinel for /path/to/another/file.js
```

Each lock file contains:
```json
{
  "agentName": "dev",
  "sessionId": "abc123def456",
  "filePath": "/Users/david/project/src/main.ts",
  "lockTime": "2024-01-01T12:00:00.000Z",
  "pid": 12345
}
```

## 🔍 Troubleshooting

### Agent Won't Start
- Check tmux is installed and running
- Verify Claude Code is available in PATH
- Check agent_state.json for name conflicts

### Hooks Not Working  
- Verify hook configuration in Claude Code settings
- Check file permissions on onAgentComplete.ts
- Test hook manually with sample JSON input
- Check Claude Code logs for hook execution

### File Locking Issues
- Run `bun fileLocking.ts cleanup` to clear expired locks
- Run `bun fileLocking.ts list` to see active locks
- Check `.file-locks/` directory for stuck lock files
- Verify file paths are absolute
- Tool execution fails with "lock conflict" - wait briefly and retry

### Watchdog False Positives
- Adjust `--stall-timeout` for longer-running tasks
- Check tmux window names match agent names
- Verify agent is actually stalled (not just waiting)

## 🧪 Testing

### Test Agent Creation
```bash
bun createAgent.ts test-agent
bun getAgentStatus.ts test-agent
bun closeAgent.ts test-agent
```

### Test File Locking
```bash
# Test manual lock operations
bun fileLocking.ts acquire ./test.txt test-agent test-session
bun fileLocking.ts list
bun fileLocking.ts release ./test.txt test-agent
```

### Test Notifications
```bash
bun notifyOrchestrator.ts system_status "Test notification"
```

## 🔮 Advanced Usage

### Multiple Orchestrators
Configure different orchestrator agents:
```bash
bun notifyOrchestrator.ts system_status "Status update" "" orchestrator-2
```

### Custom Hook Integration
Extend `onAgentComplete.ts` to:
- Parse specific task completion patterns
- Extract task results and outcomes
- Route notifications based on task type
- Update external systems or databases

### Programmatic Integration
Import utilities in other TypeScript/JavaScript code:
```typescript
import { loadAgentState, setAgentStatus, NotificationHelpers } from './utils.ts';

const state = await loadAgentState();
const notifications = new NotificationHelpers('my-orchestrator');
await notifications.systemStatus('Custom integration active');
```

## 📄 License

Part of the Gren LSP project. See parent directory for license information.