import {
  loadAgentState,
  saveAgentState,
  AgentStatus,
  type ClaudeHook,
} from "../utils.ts";
import { $ } from "bun";

// This script is called by Claude Code hooks when an agent completes a task
// It receives JSON input from stdin with hook information

try {
  // Read hook data from stdin
  const stdinText = await new Promise<string>((resolve) => {
    let data = "";
    process.stdin.on("data", (chunk) => {
      data += chunk.toString();
    });
    process.stdin.on("end", () => {
      resolve(data.trim());
    });
  });

  if (!stdinText) {
    console.error("No hook data received from stdin");
    process.exit(1);
  }

  const hookData: ClaudeHook = JSON.parse(stdinText);

  // Load current agent state
  const state = await loadAgentState();

  // Find agent by session ID
  const agentIndex = state.agents.findIndex(
    (agent) => agent.sessionId === hookData.session_id,
  );

  if (agentIndex === -1) {
    console.error(
      `Agent with session ID '${hookData.session_id}' not found in state`,
    );
    process.exit(1);
  }

  const agent = state.agents[agentIndex];

  // Update agent status to Idle and refresh lastActivity
  state.agents[agentIndex].status = AgentStatus.Idle;
  state.agents[agentIndex].lastActivity = new Date().toISOString();
  // Keep currentTask - it persists to show what the agent was last working on

  // Save updated state
  await saveAgentState(state);

  // Extract recent conversation history
  const historyExcerpt =
    await $`bun ../getAgentHistory.ts ${hookData.session_id} 50`.text();

  await $`bun ../notifyOrchestrator.ts agent_completed "Agent task completed successfully" ${agent.name}`;

  console.log(`Agent '${agent.name}' marked as idle and orchestrator notified`);
} catch (error) {
  console.error(`Error in onAgentComplete hook:`, error);
  process.exit(1);
}
