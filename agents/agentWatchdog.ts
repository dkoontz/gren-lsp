import { $ } from "bun";
import { 
  loadAgentState, 
  saveAgentState, 
  AgentStatus
} from "./utils.ts";

class AgentWatchdog {
  private isRunning = false;
  private monitorInterval: Timer | null = null;
  private agentSnapshots: Map<string, string> = new Map(); // agentName -> last output
  private stallCheckHistory: Map<string, Date> = new Map(); // agentName -> last activity time

  constructor(
    private checkIntervalSeconds: number = 30,
    private stallTimeoutMinutes: number = 5
  ) {}

  async start(): Promise<void> {
    if (this.isRunning) {
      console.log("Agent Watchdog is already running");
      return;
    }

    this.isRunning = true;
    console.log("🐕 Agent Watchdog started");
    console.log(`Check interval: ${this.checkIntervalSeconds} seconds`);
    console.log(`Stall timeout: ${this.stallTimeoutMinutes} minutes`);

    // Start monitoring agents
    this.monitorInterval = setInterval(async () => {
      try {
        await this.checkAllAgents();
      } catch (error) {
        console.error(`[${new Date().toISOString()}] Error during agent monitoring:`, error);
      }
    }, this.checkIntervalSeconds * 1000);

    console.log("Watchdog is now monitoring agents...");
    console.log("Press Ctrl+C to stop");

    // Handle shutdown
    process.on('SIGINT', () => {
      this.stop();
    });

    // Keep the process alive
    while (this.isRunning) {
      await new Promise(resolve => setTimeout(resolve, 5000));
    }
  }

  async stop(): Promise<void> {
    console.log("\n🛑 Stopping Agent Watchdog...");
    this.isRunning = false;
    
    if (this.monitorInterval) {
      clearInterval(this.monitorInterval);
      this.monitorInterval = null;
    }
    
    console.log("Agent Watchdog stopped");
    process.exit(0);
  }

  private async checkAllAgents(): Promise<void> {
    const state = await loadAgentState();
    const workingAgents = state.agents.filter(agent => agent.status === AgentStatus.Working);

    if (workingAgents.length === 0) {
      return; // No working agents to monitor
    }

    console.log(`[${new Date().toISOString()}] Monitoring ${workingAgents.length} working agents`);

    for (const agent of workingAgents) {
      await this.checkAgent(agent.name);
    }
  }

  private async checkAgent(agentName: string): Promise<void> {
    try {
      // Check if tmux window exists
      const windowExists = await this.checkTmuxWindow(agentName);
      if (!windowExists) {
        console.log(`[WATCHDOG] ⚠️  Tmux window '${agentName}' not found - agent may have crashed`);
        await this.handleCrashedAgent(agentName);
        return;
      }

      // Capture current tmux output
      const currentOutput = await this.captureTmuxOutput(agentName);
      const previousOutput = this.agentSnapshots.get(agentName);

      if (previousOutput === undefined) {
        // First time seeing this agent - record baseline
        this.agentSnapshots.set(agentName, currentOutput);
        this.stallCheckHistory.set(agentName, new Date());
        console.log(`[WATCHDOG] 📸 Baseline captured for agent '${agentName}'`);
        return;
      }

      // Check if output has changed (indicating activity)
      if (currentOutput !== previousOutput) {
        // Agent is active - update snapshots
        this.agentSnapshots.set(agentName, currentOutput);
        this.stallCheckHistory.set(agentName, new Date());
        console.log(`[WATCHDOG] ✅ Agent '${agentName}' is active`);
      } else {
        // No change detected - check if stalled
        const lastActivity = this.stallCheckHistory.get(agentName);
        if (lastActivity) {
          const now = new Date();
          const timeSinceActivity = (now.getTime() - lastActivity.getTime()) / (1000 * 60); // minutes
          
          if (timeSinceActivity > this.stallTimeoutMinutes) {
            console.log(`[WATCHDOG] 🚨 Agent '${agentName}' has been stalled for ${timeSinceActivity.toFixed(1)} minutes`);
            await this.handleStalledAgent(agentName);
          } else {
            console.log(`[WATCHDOG] ⏳ Agent '${agentName}' inactive for ${timeSinceActivity.toFixed(1)} minutes`);
          }
        }
      }
    } catch (error) {
      console.error(`[WATCHDOG] Error checking agent '${agentName}':`, error);
    }
  }

  private async checkTmuxWindow(windowName: string): Promise<boolean> {
    try {
      await $`tmux list-windows -F '#{window_name}' | grep -q "^${windowName}$"`;
      return true;
    } catch {
      return false;
    }
  }

  private async captureTmuxOutput(windowName: string): Promise<string> {
    try {
      const output = await $`tmux capture-pane -t ${windowName} -S -100 -p`.text();
      return output.trim();
    } catch (error) {
      throw new Error(`Failed to capture tmux output for '${windowName}': ${error}`);
    }
  }

  private async handleStalledAgent(agentName: string): Promise<void> {
    console.log(`[WATCHDOG] 🔧 Handling stalled agent: ${agentName}`);
    
    try {
      // Update agent status to Stalled using existing script
      await $`bun setAgentStatus.ts ${agentName} Stalled`;
      console.log(`[WATCHDOG] ✅ Agent '${agentName}' status set to Stalled`);
      
      // Clean up the stalled agent using existing script
      await $`bun cleanupStalledAgent.ts ${agentName}`;
      console.log(`[WATCHDOG] ✅ Stalled agent '${agentName}' cleanup completed`);
      
      // Clean up local tracking data
      this.cleanupLocalTracking(agentName);
      
    } catch (error) {
      console.error(`[WATCHDOG] Error handling stalled agent '${agentName}':`, error);
    }
  }

  private async handleCrashedAgent(agentName: string): Promise<void> {
    console.log(`[WATCHDOG] 🔧 Handling crashed agent: ${agentName}`);
    
    try {
      // Clean up file locks using existing script
      await $`bun cleanupLocks.ts 0`; // 0 minutes = clean all expired locks
      console.log(`[WATCHDOG] ✅ File locks cleaned up for crashed agent '${agentName}'`);
      
      // Remove from agent state
      const state = await loadAgentState();
      const agentIndex = state.agents.findIndex(agent => agent.name === agentName);
      
      if (agentIndex !== -1) {
        state.agents.splice(agentIndex, 1);
        await saveAgentState(state);
        console.log(`[WATCHDOG] ✅ Agent '${agentName}' removed from state`);
      }
      
      // Notify orchestrator using existing script
      await $`bun notifyOrchestrator.ts agent_crashed "Agent crashed unexpectedly" ${agentName}`;
      console.log(`[WATCHDOG] ✅ Orchestrator notified about crashed agent '${agentName}'`);
      
      // Clean up local tracking data
      this.cleanupLocalTracking(agentName);
      
    } catch (error) {
      console.error(`[WATCHDOG] Error handling crashed agent '${agentName}':`, error);
    }
  }

  private cleanupLocalTracking(agentName: string): void {
    // Clear local tracking data for the agent
    this.agentSnapshots.delete(agentName);
    this.stallCheckHistory.delete(agentName);
  }
}

// Main execution
async function main(): Promise<void> {
  const args = process.argv.slice(2);
  
  // Parse command line arguments
  let checkInterval = 30; // seconds
  let stallTimeout = 5; // minutes
  
  for (let i = 0; i < args.length; i += 2) {
    const flag = args[i];
    const value = args[i + 1];
    
    switch (flag) {
      case "--check-interval":
        checkInterval = parseInt(value) || 30;
        break;
      case "--stall-timeout":
        stallTimeout = parseInt(value) || 5;
        break;
      case "--help":
      case "-h":
        console.log("Agent Watchdog - Monitors working agents for stalls and crashes");
        console.log("");
        console.log("Usage: bun agentWatchdog.ts [options]");
        console.log("");
        console.log("Options:");
        console.log("  --check-interval <seconds>   How often to check agents (default: 30)");
        console.log("  --stall-timeout <minutes>    How long before marking as stalled (default: 5)");
        console.log("  --help, -h                   Show this help message");
        console.log("");
        console.log("Examples:");
        console.log("  bun agentWatchdog.ts");
        console.log("  bun agentWatchdog.ts --check-interval 60 --stall-timeout 10");
        process.exit(0);
        break;
    }
  }
  
  const watchdog = new AgentWatchdog(checkInterval, stallTimeout);
  await watchdog.start();
}

await main();