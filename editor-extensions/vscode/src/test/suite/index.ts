import * as path from 'path';
import Mocha from 'mocha';
import { glob } from 'glob';

export function run(): Promise<void> {
  // Create the mocha test with optional grep pattern
  const mochaOptions: Mocha.MochaOptions = {
    ui: 'tdd',
    color: true,
    timeout: 20000
  };
  
  // Add grep pattern if provided via environment variable
  if (process.env.MOCHA_GREP) {
    mochaOptions.grep = process.env.MOCHA_GREP;
    console.log(`Mocha configured with grep pattern: "${process.env.MOCHA_GREP}"`);
  }
  
  const mocha = new Mocha(mochaOptions);

  const testsRoot = path.resolve(__dirname, '..');

  return new Promise((c, e) => {
    glob('**/**.test.js', { cwd: testsRoot }).then((files) => {
      // Add files to the test suite
      files.forEach(f => mocha.addFile(path.resolve(testsRoot, f)));

      try {
        // Run the mocha test
        mocha.run((failures: number) => {
          if (failures > 0) {
            e(new Error(`${failures} tests failed.`));
          } else {
            c();
          }
        });
      } catch (err) {
        console.error(err);
        e(err);
      }
    }).catch((err) => {
      return e(err);
    });
  });
}