import { existsSync } from "fs";
import path from "path";

// This script helps set up Claude Code hooks for agent completion detection
// It provides instructions for configuring the hooks

const SCRIPT_DIR = import.meta.dir;
const HOOK_CONFIG_FILE = path.join(SCRIPT_DIR, "claude-hooks-config.json");
const ON_AGENT_COMPLETE_SCRIPT = path.join(SCRIPT_DIR, "onAgentComplete.ts");

async function main(): Promise<void> {
  console.log("🔧 Claude Code Hook Setup for Agent Orchestration");
  console.log("================================================");
  console.log();
  
  // Check if our hook files exist
  if (!existsSync(HOOK_CONFIG_FILE)) {
    console.error(`❌ Hook configuration file not found: ${HOOK_CONFIG_FILE}`);
    process.exit(1);
  }
  
  if (!existsSync(ON_AGENT_COMPLETE_SCRIPT)) {
    console.error(`❌ Hook handler script not found: ${ON_AGENT_COMPLETE_SCRIPT}`);
    process.exit(1);
  }
  
  console.log("✅ Hook files are ready");
  console.log();
  
  console.log("📋 Setup Instructions:");
  console.log("======================");
  console.log();
  
  console.log("1. Locate your Claude Code settings file:");
  console.log("   - macOS: ~/.config/claude-code/settings.json");
  console.log("   - Linux: ~/.config/claude-code/settings.json");
  console.log("   - Windows: %APPDATA%\\claude-code\\settings.json");
  console.log();
  
  console.log("2. Add the following hook configuration to your settings.json:");
  console.log();
  
  // Read and display the hook configuration
  const hookConfig = await Bun.file(HOOK_CONFIG_FILE).text();
  const config = JSON.parse(hookConfig);
  
  console.log("   Merge this into your existing settings.json:");
  console.log("   ```json");
  console.log(JSON.stringify(config, null, 2));
  console.log("   ```");
  console.log();
  
  console.log("3. Restart Claude Code or reload settings");
  console.log();
  
  console.log("4. Test the setup by:");
  console.log("   - Creating a test agent: bun createAgent.ts test-agent");
  console.log("   - Assigning it a task: bun setAgentStatus.ts test-agent Working");
  console.log("   - Completing a task in the agent's tmux window");
  console.log("   - Check that the agent status returns to Idle");
  console.log();
  
  console.log("🔍 Troubleshooting:");
  console.log("===================");
  console.log("- Check Claude Code logs for hook execution");
  console.log("- Ensure Bun is available in PATH");
  console.log(`- Verify script permissions: chmod +x ${ON_AGENT_COMPLETE_SCRIPT}`);
  console.log("- Test the hook script manually with sample JSON input");
  console.log();
  
  console.log("🎯 Hook Configuration File Location:");
  console.log(`   ${HOOK_CONFIG_FILE}`);
  console.log();
  console.log("🚀 Agent Complete Handler:");
  console.log(`   ${ON_AGENT_COMPLETE_SCRIPT}`);
}

await main();