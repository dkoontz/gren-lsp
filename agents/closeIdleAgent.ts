import { $ } from "bun";
import {
  AgentStatus,
  loadAgentState,
  saveAgentState,
  sendToAgent,
} from "./utils.ts";

// Check if argument is provided
if (process.argv.length !== 3) {
  console.log("Usage: bun closeAgent.ts <agent-name>");
  console.log("Example: bun closeAgent.ts dev");
  process.exit(1);
}

const agentName = process.argv[2];

try {
  // Load current agent state
  const state = await loadAgentState();

  // Check if agent exists
  const agentIndex = state.agents.findIndex(
    (agent) => agent.name === agentName,
  );
  if (agentIndex === -1) {
    console.error(`Agent '${agentName}' not found in agent state`);
    process.exit(1);
  }

  if (state.agents[agentIndex].status !== AgentStatus.Idle) {
    console.error(`Agent '${agentName}' is not idle`);
    process.exit(1);
  }
  // Send exit command to the agent
  await sendToAgent(agentName, "/exit");
  await new Promise((resolve) => setTimeout(resolve, 2000));

  // Kill the tmux window
  await $`tmux kill-window -t ${agentName}`;

  // Remove agent from state
  state.agents.splice(agentIndex, 1);
  await saveAgentState(state);

  console.log(`Agent '${agentName}' closed and removed from state`);
} catch (error) {
  console.error(`Failed to close agent '${agentName}':`, error);
  process.exit(1);
}
