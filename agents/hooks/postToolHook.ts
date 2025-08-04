#!/usr/bin/env bun
import { releaseFileLock, cleanupExpiredLocks } from "../fileLocking.ts";
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
    tool_result?: any;
  };

  // Extract tool information
  const toolName = hookData.tool_name;
  const toolArgs = hookData.tool_arguments || {};
  const sessionId = hookData.session_id.toString();

  if (!toolName) {
    // No tool was used, nothing to do
    process.exit(0);
  }

  // Check if this tool required file locking
  const requiredLocking =
    FILE_WRITING_TOOLS.has(toolName) || FILE_READING_TOOLS.has(toolName);
  if (!requiredLocking) {
    // Tool didn't require file locking
    process.exit(0);
  }

  // Extract file paths that were locked
  const filePaths = extractFilePathsFromTool(toolName, toolArgs);
  if (filePaths.length === 0) {
    // No file paths were locked
    process.exit(0);
  }

  const agentName = await getAgentName(sessionId);
  const locksReleased: string[] = [];

  // Release locks for all files
  for (const filePath of filePaths) {
    const released = await releaseFileLock(filePath, sessionId);

    if (released) {
      locksReleased.push(filePath);
      console.log(`🔓 Released lock: ${filePath}`);
    } else {
      console.warn(
        `⚠️  Could not release lock: ${filePath} (not owned or already released)`,
      );
    }
  }

  // Locks released successfully - the file locking system maintains its own state
  // Agent state management is handled by the orchestrator system

  // Periodic cleanup of expired locks (run occasionally)
  if (Math.random() < 0.1) {
    // 10% chance to run cleanup
    try {
      const cleaned = await cleanupExpiredLocks();
      if (cleaned > 0) {
        console.log(`🧹 Cleaned up ${cleaned} expired locks`);
      }
    } catch (error) {
      console.error("Error during lock cleanup:", error);
    }
  }

  process.exit(0);
} catch (error) {
  console.error("Post-tool hook error:", error);
  process.exit(1);
}
