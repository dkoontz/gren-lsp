#!/usr/bin/env bun
import { existsSync, mkdirSync } from "fs";
import { readdir } from "fs/promises";
import path from "path";
import { loadAgentState } from "./utils.ts";

const SCRIPT_DIR = import.meta.dir;
const LOCKS_DIR = path.join(SCRIPT_DIR, ".file-locks");
const LOCK_TIMEOUT_MINUTES = 10;

// Ensure locks directory exists
if (!existsSync(LOCKS_DIR)) {
  mkdirSync(LOCKS_DIR, { recursive: true });
}

/**
 * Converts a file path to a safe lock file name
 * Similar to how Claude encodes project paths
 */
function pathToLockName(filePath: string): string {
  const absolutePath = path.resolve(filePath);
  // Replace path separators and special characters with dashes
  return absolutePath
    .replace(/[\/\\:*?"<>|]/g, '-')
    .replace(/^-+|-+$/g, '') // Remove leading/trailing dashes
    .replace(/-+/g, '-'); // Collapse multiple dashes
}

/**
 * Acquires a file lock by creating a sentinel file
 * Returns true if lock acquired, false if file already locked
 */
export async function acquireFileLock(filePath: string, sessionId: string, agentName?: string): Promise<boolean> {
  const lockName = pathToLockName(filePath);
  const lockFilePath = path.join(LOCKS_DIR, lockName);
  
  // Check if lock file already exists
  if (existsSync(lockFilePath)) {
    // Check if it's expired
    const stats = await Bun.file(lockFilePath).stat();
    const lockAge = Date.now() - stats.mtime.getTime();
    const timeoutMs = LOCK_TIMEOUT_MINUTES * 60 * 1000;
    
    if (lockAge > timeoutMs) {
      // Lock is expired, remove it and acquire new lock
      try {
        await Bun.file(lockFilePath).unlink();
      } catch (error) {
        // File might have been removed by another process
      }
    } else {
      // Lock is still valid
      return false;
    }
  }
  
  // Try to create the lock file atomically
  try {
    const lockData = {
      sessionId,
      agentName: agentName || null,
      filePath: path.resolve(filePath),
      lockTime: new Date().toISOString(),
      pid: process.pid
    };
    
    // Use 'x' flag to create file exclusively (fails if exists)
    await Bun.write(lockFilePath, JSON.stringify(lockData, null, 2), { createPath: false });
    return true;
  } catch (error) {
    // File already exists or other error
    return false;
  }
}

/**
 * Releases a file lock by removing the sentinel file
 * Returns true if lock was released, false if no lock existed or not owned by session
 */
export async function releaseFileLock(filePath: string, sessionId: string): Promise<boolean> {
  const lockName = pathToLockName(filePath);
  const lockFilePath = path.join(LOCKS_DIR, lockName);
  
  if (!existsSync(lockFilePath)) {
    return false; // No lock exists
  }
  
  try {
    // Read lock data to verify ownership
    const lockData = JSON.parse(await Bun.file(lockFilePath).text());
    
    if (lockData.sessionId !== sessionId) {
      return false; // Lock not owned by this session
    }
    
    // Remove the lock file
    await Bun.file(lockFilePath).unlink();
    return true;
  } catch (error) {
    // Error reading or removing lock file
    return false;
  }
}

/**
 * Cleans up expired lock files
 * Returns the number of locks cleaned up
 */
export async function cleanupExpiredLocks(): Promise<number> {
  if (!existsSync(LOCKS_DIR)) {
    return 0;
  }
  
  const timeoutMs = LOCK_TIMEOUT_MINUTES * 60 * 1000;
  let cleanedCount = 0;
  
  try {
    const lockFiles = await readdir(LOCKS_DIR);
    
    for (const lockFile of lockFiles) {
      const lockFilePath = path.join(LOCKS_DIR, lockFile);
      
      try {
        const stats = await Bun.file(lockFilePath).stat();
        const lockAge = Date.now() - stats.mtime.getTime();
        
        if (lockAge > timeoutMs) {
          await Bun.file(lockFilePath).unlink();
          cleanedCount++;
        }
      } catch (error) {
        // Error accessing lock file, skip it
        continue;
      }
    }
  } catch (error) {
    console.error("Error during lock cleanup:", error);
  }
  
  return cleanedCount;
}

/**
 * Lists all current file locks
 * Returns array of lock information
 */
export async function listActiveLocks(): Promise<Array<{
  filePath: string;
  sessionId: string;
  agentName: string | null;
  lockTime: string;
  pid: number;
}>> {
  if (!existsSync(LOCKS_DIR)) {
    return [];
  }
  
  const locks: Array<any> = [];
  
  try {
    const lockFiles = await readdir(LOCKS_DIR);
    
    for (const lockFile of lockFiles) {
      const lockFilePath = path.join(LOCKS_DIR, lockFile);
      
      try {
        const lockData = JSON.parse(await Bun.file(lockFilePath).text());
        locks.push(lockData);
      } catch (error) {
        // Skip invalid lock files
        continue;
      }
    }
  } catch (error) {
    console.error("Error listing locks:", error);
  }
  
  return locks;
}

/**
 * Releases all locks held by a specific session
 * Returns the number of locks released
 */
export async function releaseAllLocksForSession(sessionId: string): Promise<number> {
  if (!existsSync(LOCKS_DIR)) {
    return 0;
  }
  
  let releasedCount = 0;
  
  try {
    const lockFiles = await readdir(LOCKS_DIR);
    
    for (const lockFile of lockFiles) {
      const lockFilePath = path.join(LOCKS_DIR, lockFile);
      
      try {
        const lockData = JSON.parse(await Bun.file(lockFilePath).text());
        
        if (lockData.sessionId === sessionId) {
          await Bun.file(lockFilePath).unlink();
          releasedCount++;
        }
      } catch (error) {
        // Skip invalid or inaccessible lock files
        continue;
      }
    }
  } catch (error) {
    console.error("Error releasing session locks:", error);
  }
  
  return releasedCount;
}

/**
 * Releases all locks held by a specific agent (by looking up their sessionId)
 * Returns the number of locks released
 */
export async function releaseAllLocksForAgent(agentName: string): Promise<number> {
  try {
    const agentState = await loadAgentState();
    const agent = agentState.agents.find(a => a.name === agentName);
    
    if (!agent) {
      console.error(`Agent '${agentName}' not found in agent state`);
      return 0;
    }
    
    return await releaseAllLocksForSession(agent.sessionId);
  } catch (error) {
    console.error("Error releasing agent locks:", error);
    return 0;
  }
}

// CLI interface for testing and manual operations
if (import.meta.main) {
  const args = process.argv.slice(2);
  const command = args[0];
  
  switch (command) {
    case "acquire":
      if (args.length < 3) {
        console.error("Usage: bun fileLocking.ts acquire <file-path> <session-id> [agent-name]");
        process.exit(1);
      }
      const [, filePath, sessionId, agentName] = args;
      const acquired = await acquireFileLock(filePath, sessionId, agentName);
      console.log(acquired ? "Lock acquired" : "Lock failed - file already locked");
      process.exit(acquired ? 0 : 1);
      
    case "release":
      if (args.length < 3) {
        console.error("Usage: bun fileLocking.ts release <file-path> <session-id>");
        process.exit(1);
      }
      const [, releaseFilePath, releaseSessionId] = args;
      const released = await releaseFileLock(releaseFilePath, releaseSessionId);
      console.log(released ? "Lock released" : "Lock not found or not owned by session");
      process.exit(released ? 0 : 1);
      
    case "cleanup":
      const cleaned = await cleanupExpiredLocks();
      console.log(`Cleaned up ${cleaned} expired locks`);
      break;
      
    case "list":
      const locks = await listActiveLocks();
      if (locks.length === 0) {
        console.log("No active locks");
      } else {
        console.log("Active locks:");
        for (const lock of locks) {
          const agentInfo = lock.agentName ? ` - ${lock.agentName}` : '';
          console.log(`  ${lock.filePath}${agentInfo} (${lock.sessionId}) since ${lock.lockTime}`);
        }
      }
      break;
      
    case "release-agent":
      if (args.length < 2) {
        console.error("Usage: bun fileLocking.ts release-agent <agent-name>");
        process.exit(1);
      }
      const [, releaseAgent] = args;
      const releasedCount = await releaseAllLocksForAgent(releaseAgent);
      console.log(`Released ${releasedCount} locks for agent ${releaseAgent}`);
      break;
      
    default:
      console.log("File Locking System");
      console.log("Commands:");
      console.log("  acquire <file-path> <session-id> [agent-name] - Acquire lock on file");
      console.log("  release <file-path> <session-id>              - Release lock on file");
      console.log("  cleanup                                       - Clean up expired locks");
      console.log("  list                                          - List all active locks");
      console.log("  release-agent <agent-name>                    - Release all locks for agent");
      break;
  }
}