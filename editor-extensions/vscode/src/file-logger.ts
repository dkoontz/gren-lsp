import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

export type LogSource = 'Extension' | 'LSP Server';

export class FileLogger {
    private logFilePath: string;
    private logStream: fs.WriteStream | null = null;

    constructor() {
        // Create log file in temp directory
        const tempDir = os.tmpdir();
        const logDir = path.join(tempDir, 'gren-lsp');
        this.logFilePath = path.join(logDir, 'debug.log');
        
        // Ensure log directory exists
        if (!fs.existsSync(logDir)) {
            fs.mkdirSync(logDir, { recursive: true });
        }
        
        // Clear/create log file on initialization
        this.initializeLogFile();
    }

    private initializeLogFile(): void {
        try {
            // Clear existing log file
            fs.writeFileSync(this.logFilePath, '');
            
            // Create write stream for efficient logging
            this.logStream = fs.createWriteStream(this.logFilePath, { flags: 'a' });
            
            // Log initialization
            this.log('Extension', 'File logger initialized - previous logs cleared');
            this.log('Extension', `Log file location: ${this.logFilePath}`);
        } catch (error) {
            console.error('Failed to initialize log file:', error);
        }
    }

    public log(source: LogSource, message: string): void {
        const timestamp = new Date().toISOString();
        const logEntry = `[${source}] [${timestamp}] ${message}\n`;
        
        try {
            if (this.logStream) {
                this.logStream.write(logEntry);
            } else {
                // Fallback to synchronous write if stream not available
                fs.appendFileSync(this.logFilePath, logEntry);
            }
        } catch (error) {
            console.error('Failed to write to log file:', error);
        }
    }

    public logServerOutput(data: string): void {
        // Parse server output and log each line with LSP Server prefix
        const lines = data.toString().split('\n').filter(line => line.trim());
        for (const line of lines) {
            this.log('LSP Server', line.trim());
        }
    }

    public getLogFilePath(): string {
        return this.logFilePath;
    }

    public close(): void {
        if (this.logStream) {
            this.logStream.end();
            this.logStream = null;
        }
    }

    public async readRecentLogs(maxLines: number = 100): Promise<string[]> {
        try {
            const content = fs.readFileSync(this.logFilePath, 'utf8');
            const lines = content.split('\n').filter(line => line.trim());
            return lines.slice(-maxLines);
        } catch (error) {
            console.error('Failed to read log file:', error);
            return [];
        }
    }
}

// Global logger instance
let globalLogger: FileLogger | null = null;

export function getLogger(): FileLogger {
    if (!globalLogger) {
        globalLogger = new FileLogger();
    }
    return globalLogger;
}

export function closeLogger(): void {
    if (globalLogger) {
        globalLogger.close();
        globalLogger = null;
    }
}