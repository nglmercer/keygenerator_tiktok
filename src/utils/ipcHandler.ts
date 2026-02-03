import { ipcMain, BrowserWindow } from 'electron';
import type { IpcMainInvokeEvent, IpcMainEvent } from 'electron';
import { IPC_CHANNELS, ERROR_MESSAGES } from '../constants';

/**
 * Type definition for IPC handler functions
 */
export type IpcHandlerFn<T = any> = (...args: any[]) => Promise<T> | T;

/**
 * Creates a standardized IPC handler with error handling and optional streamAPI check
 */
export function createIpcHandler<T>(
    channel: string,
    handler: IpcHandlerFn<T>,
    options: {
        requireStreamApi?: boolean;
        getStreamApi?: () => any | null;
    } = {}
): void {
    return ipcMain.handle(channel, async (event: IpcMainInvokeEvent, ...args: any[]) => {
        try {
            // Check if streamAPI is required and available
            if (options.requireStreamApi && options.getStreamApi) {
                const streamApi = options.getStreamApi();
                if (!streamApi) {
                    console.warn(`[IPC Handler] ${channel}: StreamAPI not initialized`);
                    return null;
                }
            }
            return await handler(...args);
        } catch (error: any) {
            console.error(`[IPC Handler] ${channel}:`, error);
            return {
                success: false,
                error: error.message || ERROR_MESSAGES.AUTH_FAILED,
            };
        }
    });
}

/**
 * Creates an IPC listener for simple events (not handlers)
 */
export function createIpcListener(
    channel: string,
    listener: (event: IpcMainEvent, ...args: any[]) => void
): void {
    ipcMain.on(channel, listener);
}

/**
 * Removes an IPC listener
 */
export function removeIpcListener(
    channel: string,
    listener: (event: IpcMainEvent, ...args: any[]) => void
): void {
    ipcMain.removeListener(channel, listener);
}

/**
 * Helper to create window with standardized configuration
 */
export function createStandardWindow(
    title: string,
    width: number,
    height: number,
    preloadPath: string,
    options: {
        backgroundColor?: string;
        show?: boolean;
    } = {}
): BrowserWindow {
    return new BrowserWindow({
        title,
        width,
        height,
        backgroundColor: options.backgroundColor || '#0f172a',
        show: options.show ?? true,
        webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            sandbox: false,
            preload: preloadPath,
        },
    });
}

/**
 * StreamAPI wrapper that ensures it's available before calling methods
 */
export class StreamApiWrapper {
    constructor(private streamApi: any) {}

    async call<T>(method: string, ...args: any[]): Promise<T | null> {
        if (!this.streamApi) {
            console.warn(`[StreamApiWrapper] ${method}: StreamAPI not initialized`);
            return null;
        }
        try {
            return await this.streamApi[method](...args);
        } catch (error) {
            console.error(`[StreamApiWrapper] ${method}:`, error);
            return null;
        }
    }
}

/**
 * Batch registration of IPC handlers
 */
export function registerIpcHandlers(
    handlers: Array<{
        channel: string;
        handler: IpcHandlerFn;
        requireStreamApi?: boolean;
        getStreamApi?: () => any;
    }>
): void {
    handlers.forEach(({ channel, handler, requireStreamApi, getStreamApi }) => {
        createIpcHandler(channel, handler, { requireStreamApi, getStreamApi });
    });
}
