import { $ } from "bun";
import { releaseFileLock } from "./utils.ts";

// Check if arguments are provided
if (process.argv.length !== 4) {
  console.log("Usage: bun releaseFileLock.ts <file-path> <agent-name>");
  console.log("Example: bun releaseFileLock.ts ./src/main.ts dev");
  process.exit(1);
}

const filePath = process.argv[2];
const agentName = process.argv[3];

try {
  // Verify agent exists using existing script
  try {
    await $`bun getAgentStatus.ts ${agentName}`.quiet();
  } catch {
    console.error(`Agent '${agentName}' not found`);
    process.exit(1);
  }
  
  const success = await releaseFileLock(filePath, agentName);
  
  if (success) {
    console.log(`File lock released: ${filePath}`);
  } else {
    console.log(`No lock found or not owned by agent '${agentName}': ${filePath}`);
  }
} catch (error) {
  console.error(`Failed to release file lock:`, error);
  process.exit(1);
}