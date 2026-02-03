import fs from 'fs';
import path from 'path';
import { app } from 'electron';

export type JsonData = Record<string, unknown>;
export type JsonArray = unknown[];
export type JsonValue = JsonData | JsonArray | string | number | boolean | null;

/**
 * Get the application base path
 * In development: returns the project root
 * In production: returns the packaged app resources path
 */
export function getAppBasePath(): string {
    // Check if we're in development mode
    const isDev = !app.isPackaged;
    
    if (isDev) {
        // In development, use the current working directory
        return process.cwd();
    } else {
        // In production, use the resources path
        return process.resourcesPath;
    }
}

/**
 * Resolve a path relative to the application base path
 */
export function resolveAppPath(...pathSegments: string[]): string {
    return path.resolve(getAppBasePath(), ...pathSegments);
}

/**
 * File utilities to avoid repeated file operations
 */
export const FileUtils = {
    /**
     * Read JSON file safely
     */
    readJson<T extends JsonData>(filename: string, defaultData: T): T {
        const filePath = path.resolve(process.cwd(), filename);
        if (fs.existsSync(filePath)) {
            try {
                const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
                if (typeof data === 'object' && data !== null && !Array.isArray(data)) {
                    return { ...defaultData, ...data };
                }
                return data as T;
            } catch (error) {
                console.error(`[FileUtils] Failed to read ${filename}:`, error);
            }
        }
        return defaultData;
    },

    /**
     * Read JSON array file safely
     */
    readJsonArray<T extends JsonArray>(filename: string, defaultData: T): T {
        const filePath = path.resolve(process.cwd(), filename);
        if (fs.existsSync(filePath)) {
            try {
                const data = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
                if (Array.isArray(data)) {
                    return data as T;
                }
            } catch (error) {
                console.error(`[FileUtils] Failed to read ${filename}:`, error);
            }
        }
        return defaultData;
    },

    /**
     * Write JSON file safely
     */
    writeJson(filename: string, data: JsonValue): void {
        const filePath = path.resolve(process.cwd(), filename);
        try {
            fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
        } catch (error) {
            console.error(`[FileUtils] Failed to write ${filename}:`, error);
        }
    },

    /**
     * Check if file exists
     */
    exists(filename: string): boolean {
        const filePath = path.resolve(process.cwd(), filename);
        return fs.existsSync(filePath);
    },

    /**
     * Read file content
     */
    read(filename: string): string | null {
        const filePath = path.resolve(process.cwd(), filename);
        if (fs.existsSync(filePath)) {
            try {
                return fs.readFileSync(filePath, 'utf-8');
            } catch (error) {
                console.error(`[FileUtils] Failed to read ${filename}:`, error);
            }
        }
        return null;
    },

    /**
     * Write file content
     */
    write(filename: string, content: string): void {
        const filePath = path.resolve(process.cwd(), filename);
        try {
            fs.writeFileSync(filePath, content);
        } catch (error) {
            console.error(`[FileUtils] Failed to write ${filename}:`, error);
        }
    },
};

/**
 * Token storage utility
 */
export class TokenStorage {
    private tokenPath: string;

    constructor(filename: string = 'tokens.json') {
        this.tokenPath = path.resolve(process.cwd(), filename);
    }

    get(): string | null {
        if (fs.existsSync(this.tokenPath)) {
            try {
                const data = JSON.parse(fs.readFileSync(this.tokenPath, 'utf-8'));
                return data.oauth_token || null;
            } catch (error) {
                console.error('[TokenStorage] Failed to load tokens:', error);
            }
        }
        return null;
    }

    save(data: Record<string, unknown>): void {
        FileUtils.writeJson(this.tokenPath, data);
    }
}

/**
 * Config storage utility
 */
export class ConfigStorage<T extends JsonData = Record<string, unknown>> {
    private configPath: string;
    private defaultConfig: T;

    constructor(filename: string = 'config.json', defaultConfig: T) {
        this.configPath = path.resolve(process.cwd(), filename);
        this.defaultConfig = defaultConfig;
    }

    load(): T {
        return FileUtils.readJson(this.configPath, this.defaultConfig);
    }

    save(config: Partial<T>): void {
        const current = this.load();
        const merged = { ...current, ...config };
        FileUtils.writeJson(this.configPath, merged);
    }
}
