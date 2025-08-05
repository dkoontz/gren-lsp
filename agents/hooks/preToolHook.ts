#!/usr/bin/env bun
import { acquireFileLock, releaseFileLock } from "../fileLocking.ts";
import {
  getAgentName,
  extractFilePathsFromTool,
  FILE_WRITING_TOOLS,
  FILE_READING_TOOLS,
  type ClaudeHook,
} from "../utils.ts";

try {
  // Read hook data from stdin
  const hookData = (await new Response(process.stdin).json()) as ClaudeHook & {
    tool_name?: string;
    tool_arguments?: any;
  };

  // Extract tool information
  const toolName = hookData.tool_name;
  const toolArgs = hookData.tool_arguments || {};
  const sessionId = hookData.session_id.toString();

  if (!toolName) {
    // No tool being used, nothing to do
    process.exit(0);
  }

  // Check if this tool requires file locking
  const requiresLocking =
    FILE_WRITING_TOOLS.has(toolName) || FILE_READING_TOOLS.has(toolName);
  if (!requiresLocking) {
    // Tool doesn't require file locking
    process.exit(0);
  }

  // Extract file paths that need locking
  const filePaths = extractFilePathsFromTool(toolName, toolArgs);
  if (filePaths.length === 0) {
    // No file paths to lock
    process.exit(0);
  }

  const agentName = await getAgentName(sessionId);
  const locksAcquired: string[] = [];
  let lockFailed = false;
  let failedAcquiringLockForFile = "";

  // Try to acquire locks for all files
  for (const filePath of filePaths) {
    const acquired = await acquireFileLock(filePath, sessionId, agentName);

    if (acquired) {
      locksAcquired.push(filePath);
      console.log(`🔒 Acquired lock: ${filePath}`);
    } else {
      lockFailed = true;
      failedAcquiringLockForFile = filePath;
      break;
    }
  }

  if (lockFailed) {
    // Release any locks we managed to acquire
    for (const lockedPath of locksAcquired) {
      await releaseFileLock(lockedPath, sessionId);
    }

    console.error(
      `Failed to acquire lock for file ${failedAcquiringLockForFile}, cannot proceed with ${toolName} operation. Please wait for a short time and then try again.`,
    );
    process.exit(1);
  }

  process.exit(0);
} catch (error) {
  console.error("Pre-tool hook error:", error);
  process.exit(1);
}
