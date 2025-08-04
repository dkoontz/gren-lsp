import { sendToAgent } from "./utils.ts";
import { $ } from "bun";

// Types for different notification events
export type NotificationType = 
  | "agent_completed"
  | "agent_stalled" 
  | "agent_crashed"
  | "general";

/**
 * Send a notification to the orchestrator tmux window
 */
export async function notifyOrchestrator(
  type: NotificationType,
  message: string,
  agentName?: string
): Promise<boolean> {
  try {
    // Check if orchestrator tmux window exists
    const windows = await $`tmux list-windows -F "#{window_name}"`.text();
    const windowList = windows.trim().split('\n');
    
    if (!windowList.includes("orchestrator")) {
      console.log(`Orchestrator window 'orchestrator' not found`);
      console.log(`[ORCHESTRATOR NOTIFICATION] ${type.toUpperCase()}: ${message}${agentName ? ` (Agent: ${agentName})` : ''}`);
      return false;
    }
    
    // Format notification message
    const timestamp = new Date().toISOString();
    const formattedType = type.toUpperCase().replace('_', ' ');
    
    let notification = `🤖 AGENT NOTIFICATION\n`;
    notification += `Time: ${timestamp}\n`;
    notification += `Type: ${formattedType}\n`;
    
    if (agentName) {
      notification += `Agent: ${agentName}\n`;
    }
    
    notification += `Message: ${message}\n`;
    notification += `${"=".repeat(40)}`;
    
    // Send to orchestrator window
    await sendToAgent("orchestrator", notification);
    console.log(`✅ Notification sent to orchestrator window 'orchestrator'`);
    return true;
    
  } catch (error) {
    console.error(`Failed to notify orchestrator:`, error);
    console.log(`[ORCHESTRATOR NOTIFICATION] ${type.toUpperCase()}: ${message}${agentName ? ` (Agent: ${agentName})` : ''}`);
    return false;
  }
}

// CLI execution
const args = process.argv.slice(2);

if (args.length < 2) {
  console.log("Usage: bun notifyOrchestrator.ts <type> <message> [agent-name]");
  console.log("");
  console.log("Types:");
  console.log("  agent_completed - Agent finished a task");
  console.log("  agent_stalled   - Agent was detected as stalled");
  console.log("  agent_crashed   - Agent crashed unexpectedly");
  console.log("  general         - General message");
  console.log("");
  console.log("Examples:");
  console.log("  bun notifyOrchestrator.ts general 'System startup complete'");
  console.log("  bun notifyOrchestrator.ts agent_completed 'Task finished successfully' dev");
  console.log("  bun notifyOrchestrator.ts agent_stalled 'Agent stopped responding' worker1");
  process.exit(1);
}

const type = args[0] as NotificationType;
const message = args[1];
const agentName = args[2];

// Validate notification type
const validTypes: NotificationType[] = ["agent_completed", "agent_stalled", "agent_crashed", "general"];
if (!validTypes.includes(type)) {
  console.error(`Invalid notification type: ${type}`);
  console.error(`Valid types: ${validTypes.join(", ")}`);
  process.exit(1);
}

const success = await notifyOrchestrator(type, message, agentName);

if (!success) {
  console.log("⚠️  Notification fallback used (orchestrator window not found)");
}