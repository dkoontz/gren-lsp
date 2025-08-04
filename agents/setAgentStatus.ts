import { loadAgentState, saveAgentState, AgentStatus } from "./utils.ts";

// Check if both arguments are provided
if (process.argv.length !== 4) {
  console.log("Usage: bun setAgentStatus.ts <agent-name> <status>");
  console.log("Example: bun setAgentStatus.ts dev Working");
  console.log("Valid statuses: Idle, Working, Stalled");
  process.exit(1);
}

const agentName = process.argv[2];
const statusString = process.argv[3];

// Validate status string
const validStatuses = ["Idle", "Working", "Stalled"];
if (!validStatuses.includes(statusString)) {
  console.error(`Invalid status '${statusString}'. Valid statuses: ${validStatuses.join(", ")}`);
  process.exit(1);
}

// Convert string to enum
const status = AgentStatus[statusString as keyof typeof AgentStatus];

try {
  // Load current agent state
  const state = await loadAgentState();
  
  // Find the agent
  const agentIndex = state.agents.findIndex(agent => agent.name === agentName);
  
  if (agentIndex === -1) {
    console.error(`Agent '${agentName}' not found`);
    process.exit(1);
  }
  
  // Update agent status and timestamp
  state.agents[agentIndex].status = status;
  state.agents[agentIndex].lastActivity = new Date().toISOString();
  
  // Save updated state
  await saveAgentState(state);
  
  console.log(`Agent '${agentName}' status updated to ${statusString}`);
} catch (error) {
  console.error(`Failed to update status for agent '${agentName}':`, error);
  process.exit(1);
}