import { $ } from "bun";
import path from "path";
import {
  loadAgentState,
  saveAgentState,
  AgentStatus,
  type Agent,
} from "./utils.ts";

const SCRIPT_DIR = import.meta.dir;

// if argument is provided
if (process.argv.length !== 3) {
  console.log("Usage: bun createAgent.ts <agent-name>");
  console.log("Example: bun createAgent.ts dev");
  process.exit(1);
}

const agentName = process.argv[2];

try {
  // Load current agent state
  const state = await loadAgentState();

  // Check if agent already exists
  const existingAgent = state.agents.find((agent) => agent.name === agentName);
  if (existingAgent) {
    console.error(
      `Agent '${agentName}' already exists with session ID: ${existingAgent.sessionId}`,
    );
    process.exit(1);
  }

  // Create new tmux window
  await $`tmux new-window -n ${agentName}`;

  // Start claude in the window
  await $`bun sendToAgent.ts ${agentName} "claude --dangerously-skip-permissions"`;
  // Wait for Claude to start up
  await new Promise((resolve) => setTimeout(resolve, 2000));

  // Get status to capture session ID
  await $`bun sendToAgent.ts ${agentName} "/status"`;
  // Wait for Claude to process the command
  await new Promise((resolve) => setTimeout(resolve, 1000));

  // Extract session ID from tmux output
  const output = await $`tmux capture-pane -t ${agentName} -S -50 -p`.text();
  const sessionIdMatch = output.match(/Session ID: (.+)/);
  if (!sessionIdMatch) {
    throw new Error(`Could not extract session ID for agent ${agentName}`);
  }
  const sessionId = sessionIdMatch[1].trim();

  // Close status info pane
  await $`bun sendToAgent.ts ${agentName} ""`;

  // Add agent to state
  const newAgent: Agent = {
    name: agentName,
    sessionId: sessionId,
    status: AgentStatus.Idle,
    lastActivity: new Date().toISOString(),
    fileLocksHeld: [],
  };

  state.agents.push(newAgent);
  await saveAgentState(state);

  console.log(
    `Agent '${agentName}' initialized successfully with session ID: ${sessionId}`,
  );
} catch (error) {
  console.error(`Failed to initialize agent '${agentName}':`, error);
  process.exit(1);
}
