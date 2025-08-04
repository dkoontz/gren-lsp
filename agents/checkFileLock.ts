import { checkFileLock } from "./utils.ts";

// Check if argument is provided
if (process.argv.length !== 3) {
  console.log("Usage: bun checkFileLock.ts <file-path>");
  console.log("Example: bun checkFileLock.ts ./src/main.ts");
  process.exit(1);
}

const filePath = process.argv[2];

try {
  const lock = await checkFileLock(filePath);
  
  if (lock) {
    console.log(`File '${filePath}' is LOCKED`);
    console.log(`Agent: ${lock.agentName}`);
    console.log(`Session: ${lock.sessionId}`);
    console.log(`Since: ${lock.lockTime}`);
    console.log(`Operation: ${lock.operation}`);
  } else {
    console.log(`File '${filePath}' is AVAILABLE`);
  }
} catch (error) {
  console.error(`Failed to check file lock:`, error);
  process.exit(1);
}