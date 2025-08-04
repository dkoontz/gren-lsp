import { loadAgentState, parseConversationFile, formatProcessedMessage, findConversationFile } from "./utils.ts";
import { existsSync } from "fs";
import path from "path";

// Check arguments
if (process.argv.length < 3 || process.argv.length > 4) {
  console.log("Usage: bun getAgentHistory.ts <session-id> [line-count]");
  console.log("       bun getAgentHistory.ts <agent-name> [line-count]");
  console.log("Example: bun getAgentHistory.ts abc123def456 50");
  console.log("Example: bun getAgentHistory.ts dev 100");
  console.log("");
  console.log(
    "line-count: Number of recent messages to retrieve (default: 50, max: 200)",
  );
  process.exit(1);
}

const identifier = process.argv[2]; // Could be session ID or agent name
const lineCount = Math.min(parseInt(process.argv[3] || "50"), 200);

async function getConversationHistory(
  conversationPath: string,
  lineCount: number,
): Promise<string> {
  if (!existsSync(conversationPath)) {
    return "Conversation file not found";
  }

  try {
    const parsedConversation = await parseConversationFile(conversationPath);
    
    if (parsedConversation.totalMessages === 0) {
      return "No messages found in conversation";
    }

    // Get recent messages
    const recentMessages = parsedConversation.messages.slice(-lineCount);

    // Format messages for display using type-safe formatter
    const formattedMessages = recentMessages.map((message, index) => {
      // Update the index for display
      const displayMessage = { ...message, index };
      return formatProcessedMessage(displayMessage);
    });

    return formattedMessages.join("\n\n");
  } catch (error) {
    return `Error parsing conversation history: ${error}`;
  }
}


try {
  let sessionId = identifier;

  // Check if identifier is an agent name (try to resolve to session ID)
  if (identifier.length < 20) {
    // Session IDs are typically longer
    const state = await loadAgentState();
    const agent = state.agents.find((agent) => agent.name === identifier);

    if (agent) {
      sessionId = agent.sessionId;
      console.log(`Resolved agent '${identifier}' to session ID: ${sessionId}`);
    } else {
      console.log(
        `No agent found with name '${identifier}', treating as session ID`,
      );
    }
  }

  // Find the conversation file
  const conversationFile = await findConversationFile(sessionId);

  if (!conversationFile) {
    console.error(`No conversation history found for session ID: ${sessionId}`);
    console.error("Checked directories under: ~/.claude/projects/*/");
    process.exit(1);
  }

  // Parse and display the history (content only)
  const history = await getConversationHistory(conversationFile, lineCount);
  console.log(history);
} catch (error) {
  console.error(`Failed to retrieve agent history:`, error);
  process.exit(1);
}
