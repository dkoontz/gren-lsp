import { runTests } from '@vscode/test-electron';
import * as path from 'path';

interface TestOptions {
  grep?: string;
}

function parseCommandLineArgs(): TestOptions {
  const args = process.argv.slice(2);
  const options: TestOptions = {};
  
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--grep' && i + 1 < args.length) {
      options.grep = args[i + 1];
      i++; // Skip the next argument as it's the grep pattern
    } else if (arg.startsWith('--grep=')) {
      options.grep = arg.substring('--grep='.length);
    }
  }
  
  return options;
}

async function main() {
  try {
    // Parse command line arguments
    const testOptions = parseCommandLineArgs();
    
    // The folder containing the Extension Manifest package.json
    // Passed to `--extensionDevelopmentPath`
    const extensionDevelopmentPath = path.resolve(__dirname, '../../');

    // The path to test runner
    // Passed to --extensionTestsPath
    const extensionTestsPath = path.resolve(__dirname, './suite/index');

    // The path to the workspace folder to open during testing
    const testWorkspace = path.resolve(__dirname, '../../test-workspace');

    // Pass test options through environment variables since runTests doesn't support custom options
    if (testOptions.grep) {
      process.env.MOCHA_GREP = testOptions.grep;
      console.log(`Running tests with grep pattern: "${testOptions.grep}"`);
    }

    // Download VS Code, unzip it and run the integration test
    await runTests({ 
      extensionDevelopmentPath, 
      extensionTestsPath,
      launchArgs: [testWorkspace, '--disable-extensions']
    });
  } catch (err) {
    console.error('Failed to run tests');
    process.exit(1);
  }
}

main();