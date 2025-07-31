/**
 * Test logging utility that supports different verbosity levels
 * Controlled by TEST_LOG_LEVEL environment variable
 */

export enum LogLevel {
  QUIET = 0,    // Only show errors and test failures
  INFO = 1,     // Show success messages and important info
  DEBUG = 2,    // Show progress messages and debug info
  VERBOSE = 3   // Show all messages including detailed monitoring
}

class TestLogger {
  private level: LogLevel;

  constructor() {
    const envLevel = process.env.TEST_LOG_LEVEL?.toLowerCase();
    switch (envLevel) {
      case 'verbose':
        this.level = LogLevel.VERBOSE;
        break;
      case 'debug':
        this.level = LogLevel.DEBUG;
        break;
      case 'info':
        this.level = LogLevel.INFO;
        break;
      case 'quiet':
      default:
        this.level = LogLevel.QUIET;
        break;
    }
  }

  /**
   * Always shown - for errors and test failures
   */
  error(message: string, ...args: any[]): void {
    console.log(message, ...args);
  }

  /**
   * Shown at INFO level and above - for important success messages
   */
  info(message: string, ...args: any[]): void {
    if (this.level >= LogLevel.INFO) {
      console.log(message, ...args);
    }
  }

  /**
   * Shown at DEBUG level and above - for progress and debugging
   */
  debug(message: string, ...args: any[]): void {
    if (this.level >= LogLevel.DEBUG) {
      console.log(message, ...args);
    }
  }

  /**
   * Shown at VERBOSE level only - for detailed monitoring
   */
  verbose(message: string, ...args: any[]): void {
    if (this.level >= LogLevel.VERBOSE) {
      console.log(message, ...args);
    }
  }

  /**
   * Get current log level for conditional logging
   */
  getLevel(): LogLevel {
    return this.level;
  }

  /**
   * Check if a specific level is enabled
   */
  isEnabled(level: LogLevel): boolean {
    return this.level >= level;
  }
}

// Export singleton instance
export const testLogger = new TestLogger();