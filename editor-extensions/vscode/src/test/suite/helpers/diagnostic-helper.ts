import * as vscode from 'vscode';
import * as assert from 'assert';

export interface DiagnosticExpectation {
  message?: string | RegExp;
  severity?: vscode.DiagnosticSeverity;
  line?: number;
  character?: number;
  startLine?: number;
  startCharacter?: number;
  endLine?: number;
  endCharacter?: number;
  source?: string;
  code?: string | number;
}

export class DiagnosticHelper {
  
  /**
   * Assert that diagnostics match expectations
   */
  static assertDiagnostics(
    actual: vscode.Diagnostic[], 
    expected: DiagnosticExpectation[]
  ): void {
    assert.strictEqual(
      actual.length, 
      expected.length, 
      `Expected ${expected.length} diagnostics, but got ${actual.length}. ` +
      `Actual diagnostics: ${JSON.stringify(actual.map(d => ({
        message: d.message,
        severity: d.severity,
        range: d.range
      })), null, 2)}`
    );

    for (let i = 0; i < expected.length; i++) {
      const actualDiag = actual[i];
      const expectedDiag = expected[i];

      this.assertDiagnosticMatches(actualDiag, expectedDiag, i);
    }
  }

  /**
   * Assert that a single diagnostic matches expectations
   */
  static assertDiagnosticMatches(
    actual: vscode.Diagnostic, 
    expected: DiagnosticExpectation, 
    index?: number
  ): void {
    const prefix = index !== undefined ? `Diagnostic ${index}: ` : '';

    // Check message
    if (expected.message !== undefined) {
      if (typeof expected.message === 'string') {
        assert.ok(
          actual.message.includes(expected.message),
          `${prefix}Expected message to contain "${expected.message}", but got "${actual.message}"`
        );
      } else {
        assert.ok(
          expected.message.test(actual.message),
          `${prefix}Expected message to match pattern ${expected.message}, but got "${actual.message}"`
        );
      }
    }

    // Check severity
    if (expected.severity !== undefined) {
      assert.strictEqual(
        actual.severity, 
        expected.severity,
        `${prefix}Expected severity ${expected.severity}, but got ${actual.severity}`
      );
    }

    // Check range
    if (expected.line !== undefined || expected.character !== undefined) {
      if (expected.line !== undefined) {
        assert.strictEqual(
          actual.range.start.line,
          expected.line,
          `${prefix}Expected line ${expected.line}, but got ${actual.range.start.line}`
        );
      }
      if (expected.character !== undefined) {
        assert.strictEqual(
          actual.range.start.character,
          expected.character,
          `${prefix}Expected character ${expected.character}, but got ${actual.range.start.character}`
        );
      }
    }

    // Check detailed range
    if (expected.startLine !== undefined) {
      assert.strictEqual(
        actual.range.start.line,
        expected.startLine,
        `${prefix}Expected start line ${expected.startLine}, but got ${actual.range.start.line}`
      );
    }
    if (expected.startCharacter !== undefined) {
      assert.strictEqual(
        actual.range.start.character,
        expected.startCharacter,
        `${prefix}Expected start character ${expected.startCharacter}, but got ${actual.range.start.character}`
      );
    }
    if (expected.endLine !== undefined) {
      assert.strictEqual(
        actual.range.end.line,
        expected.endLine,
        `${prefix}Expected end line ${expected.endLine}, but got ${actual.range.end.line}`
      );
    }
    if (expected.endCharacter !== undefined) {
      assert.strictEqual(
        actual.range.end.character,
        expected.endCharacter,
        `${prefix}Expected end character ${expected.endCharacter}, but got ${actual.range.end.character}`
      );
    }

    // Check source
    if (expected.source !== undefined) {
      assert.strictEqual(
        actual.source,
        expected.source,
        `${prefix}Expected source "${expected.source}", but got "${actual.source}"`
      );
    }

    // Check code
    if (expected.code !== undefined) {
      assert.strictEqual(
        actual.code,
        expected.code,
        `${prefix}Expected code "${expected.code}", but got "${actual.code}"`
      );
    }
  }

  /**
   * Assert that there are no diagnostics
   */
  static assertNoDiagnostics(diagnostics: vscode.Diagnostic[]): void {
    assert.strictEqual(
      diagnostics.length, 
      0, 
      `Expected no diagnostics, but got ${diagnostics.length}: ${JSON.stringify(
        diagnostics.map(d => d.message), 
        null, 
        2
      )}`
    );
  }

  /**
   * Assert that diagnostics contain specific errors
   */
  static assertContainsError(
    diagnostics: vscode.Diagnostic[], 
    expectedMessage: string | RegExp
  ): void {
    const errors = diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
    
    const hasMatchingError = errors.some(error => {
      if (typeof expectedMessage === 'string') {
        return error.message.includes(expectedMessage);
      } else {
        return expectedMessage.test(error.message);
      }
    });

    assert.ok(
      hasMatchingError,
      `Expected to find error containing "${expectedMessage}" in diagnostics. ` +
      `Actual errors: ${JSON.stringify(errors.map(e => e.message), null, 2)}`
    );
  }

  /**
   * Assert that diagnostics contain specific warnings
   */
  static assertContainsWarning(
    diagnostics: vscode.Diagnostic[], 
    expectedMessage: string | RegExp
  ): void {
    const warnings = diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Warning);
    
    const hasMatchingWarning = warnings.some(warning => {
      if (typeof expectedMessage === 'string') {
        return warning.message.includes(expectedMessage);
      } else {
        return expectedMessage.test(warning.message);
      }
    });

    assert.ok(
      hasMatchingWarning,
      `Expected to find warning containing "${expectedMessage}" in diagnostics. ` +
      `Actual warnings: ${JSON.stringify(warnings.map(w => w.message), null, 2)}`
    );
  }

  /**
   * Wait for diagnostics to change and match expectations
   */
  static async waitForDiagnosticsToMatch(
    uri: vscode.Uri,
    expected: DiagnosticExpectation[],
    timeoutMs: number = 10000
  ): Promise<vscode.Diagnostic[]> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeoutMs) {
      const diagnostics = vscode.languages.getDiagnostics(uri);
      
      try {
        this.assertDiagnostics(diagnostics, expected);
        return diagnostics; // Success
      } catch (error) {
        // Continue waiting
        await new Promise(resolve => setTimeout(resolve, 200));
      }
    }
    
    // Timeout - provide detailed error
    const actualDiagnostics = vscode.languages.getDiagnostics(uri);
    throw new Error(
      `Timeout waiting for diagnostics to match expectations.\n` +
      `Expected: ${JSON.stringify(expected, null, 2)}\n` +
      `Actual: ${JSON.stringify(actualDiagnostics.map(d => ({
        message: d.message,
        severity: d.severity,
        range: {
          start: { line: d.range.start.line, character: d.range.start.character },
          end: { line: d.range.end.line, character: d.range.end.character }
        },
        source: d.source,
        code: d.code
      })), null, 2)}`
    );
  }

  /**
   * Log diagnostics for debugging
   */
  static logDiagnostics(uri: vscode.Uri, label: string = 'Diagnostics'): void {
    const diagnostics = vscode.languages.getDiagnostics(uri);
    console.log(`📋 ${label} for ${uri.toString()} (${diagnostics.length} items):`);
    
    if (diagnostics.length === 0) {
      console.log('  (no diagnostics)');
      return;
    }

    diagnostics.forEach((diag, index) => {
      const severityName = this.getSeverityName(diag.severity);
      const range = `${diag.range.start.line}:${diag.range.start.character}-${diag.range.end.line}:${diag.range.end.character}`;
      console.log(`  ${index + 1}. [${severityName}] ${diag.message} (${range})`);
      if (diag.source) {
        console.log(`     Source: ${diag.source}`);
      }
      if (diag.code) {
        console.log(`     Code: ${diag.code}`);
      }
    });
  }

  /**
   * Get human-readable severity name
   */
  static getSeverityName(severity: vscode.DiagnosticSeverity): string {
    switch (severity) {
      case vscode.DiagnosticSeverity.Error: return 'ERROR';
      case vscode.DiagnosticSeverity.Warning: return 'WARNING';
      case vscode.DiagnosticSeverity.Information: return 'INFO';
      case vscode.DiagnosticSeverity.Hint: return 'HINT';
      default: return 'UNKNOWN';
    }
  }

  /**
   * Create a diagnostic expectation for common Gren compiler errors
   */
  static grenCompileError(message: string | RegExp, line?: number, character?: number): DiagnosticExpectation {
    return {
      message,
      severity: vscode.DiagnosticSeverity.Error,
      line,
      character,
      source: 'gren'
    };
  }

  /**
   * Create a diagnostic expectation for syntax errors
   */
  static grenSyntaxError(message: string | RegExp, line?: number, character?: number): DiagnosticExpectation {
    return {
      message,
      severity: vscode.DiagnosticSeverity.Error,
      line,
      character,
      source: 'gren'
    };
  }

  /**
   * Create a diagnostic expectation for type errors
   */
  static grenTypeError(message: string | RegExp, line?: number, character?: number): DiagnosticExpectation {
    return {
      message,
      severity: vscode.DiagnosticSeverity.Error,
      line,
      character,
      source: 'gren'
    };
  }
}