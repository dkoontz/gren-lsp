import { sendToAgent } from "./utils.ts";

// Check if both arguments are provided
if (process.argv.length !== 4) {
  console.log("Usage: bun sendToAgent.ts <agent-name> <command-text>");
  console.log("Example: bun sendToAgent.ts my-window 'echo hello'");
  process.exit(1);
}

const windowName = process.argv[2];
const commandText = process.argv[3];

try {
  await sendToAgent(windowName, commandText);
} catch (error) {
  console.error(`Failed to send command to agent '${windowName}':`, error);
  process.exit(1);
}
