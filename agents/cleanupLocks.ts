import { cleanupExpiredLocks } from "./utils.ts";

// Optional timeout argument
const timeoutMinutes = process.argv[2] ? parseInt(process.argv[2]) : 10;

if (isNaN(timeoutMinutes) || timeoutMinutes <= 0) {
  console.log("Usage: bun cleanupLocks.ts [timeout-minutes]");
  console.log("Example: bun cleanupLocks.ts 15");
  console.log("Default timeout: 10 minutes");
  process.exit(1);
}

try {
  console.log(`Cleaning up file locks older than ${timeoutMinutes} minutes...`);
  
  const cleanedCount = await cleanupExpiredLocks(timeoutMinutes);
  
  if (cleanedCount > 0) {
    console.log(`✅ Cleaned up ${cleanedCount} expired file locks`);
  } else {
    console.log(`✅ No expired locks found`);
  }
} catch (error) {
  console.error(`Failed to cleanup locks:`, error);
  process.exit(1);
}