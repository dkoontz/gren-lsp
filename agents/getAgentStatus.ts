import { loadAgentState, AgentStatus } from "./utils.ts";

// Check if argument is provided
if (process.argv.length !== 3) {
  console.log("Usage: bun getAgentStatus.ts <agent-name>");
  console.log("Example: bun getAgentStatus.ts dev");
  process.exit(1);
}

const agentName = process.argv[2];

try {
  // Load current agent state
  const state = await loadAgentState();
  
  // Find the agent
  const agent = state.agents.find(agent => agent.name === agentName);
  
  if (!agent) {
    console.error(`Agent '${agentName}' not found`);
    process.exit(1);
  }
  
  // Return the status as a string
  const statusString = AgentStatus[agent.status];
  console.log(statusString);
} catch (error) {
  console.error(`Failed to get status for agent '${agentName}':`, error);
  process.exit(1);
}