import { $ } from "bun";
import { loadAgentState, saveAgentState, AgentStatus, sendToAgent } from "./utils.ts";

// Check if argument is provided
if (process.argv.length !== 3) {
  console.log("Usage: bun cleanupStalledAgent.ts <agent-name>");
  console.log("Example: bun cleanupStalledAgent.ts dev");
  console.log("");
  console.log("This script handles cleanup of stalled agents by:");
  console.log("- Verifying the agent is stalled");
  console.log("- Killing the agent process");
  console.log("- Cleaning up resources (locks, tmux windows)");
  console.log("- Notifying the orchestrator");
  process.exit(1);
}

const agentName = process.argv[2];

async function killAgent(agentName: string): Promise<void> {
  try {
    // Try to gracefully exit first
    await sendToAgent(agentName, "/exit");
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Force kill the tmux window
    await $`tmux kill-window -t ${agentName}`;
    console.log(`✅ Agent '${agentName}' terminated`);
  } catch (error) {
    console.error(`Error killing agent '${agentName}':`, error);
  }
}

try {
  // Load current agent state
  const state = await loadAgentState();
  
  // Find the agent
  const agentIndex = state.agents.findIndex(agent => agent.name === agentName);
  
  if (agentIndex === -1) {
    console.error(`Agent '${agentName}' not found in agent state`);
    process.exit(1);
  }
  
  const agent = state.agents[agentIndex];
  
  // Verify agent is stalled
  if (agent.status !== AgentStatus.Stalled) {
    console.error(`Agent '${agentName}' is not stalled (current status: ${AgentStatus[agent.status]})`);
    console.error("This script can only cleanup stalled agents");
    process.exit(1);
  }
  
  console.log(`🔧 Cleaning up stalled agent: ${agentName}`);
  
  // Kill the stalled agent
  await killAgent(agentName);
  
  // Clean up file locks using existing script
  console.log(`🔒 Cleaning up file locks for agent '${agentName}'...`);
  try {
    await $`bun cleanupLocks.ts 0`; // 0 minutes = clean all expired locks
    console.log(`✅ File locks cleanup completed`);
  } catch (error) {
    console.error(`Warning: Failed to cleanup locks:`, error);
  }
  
  // Remove from agent state
  state.agents.splice(agentIndex, 1);
  await saveAgentState(state);
  console.log(`✅ Agent '${agentName}' removed from state`);
  
  // Notify orchestrator using existing script
  console.log(`📢 Notifying orchestrator about stalled agent...`);
  try {
    await $`bun notifyOrchestrator.ts agent_stalled "Agent was stalled and has been cleaned up" ${agentName}`;
    console.log(`✅ Orchestrator notified`);
  } catch (error) {
    console.error(`Warning: Failed to notify orchestrator:`, error);
  }
  
  console.log(`🎉 Stalled agent '${agentName}' cleanup completed successfully`);
  
} catch (error) {
  console.error(`Failed to cleanup stalled agent '${agentName}':`, error);
  process.exit(1);
}