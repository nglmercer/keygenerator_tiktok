import path from 'path';
import fs from 'fs';
import { spawn } from 'child_process';
import * as electron from 'electron';

// Self-relaunch in Electron if running in a non-electron environment (like Bun)
if (!process.versions.electron) {
    const electronPath = path.resolve(process.cwd(), 'node_modules', '.bin', 'electron');
    const args = process.argv.slice(1);

    spawn(electronPath, args, {
        stdio: 'inherit',
        env: { ...process.env, ELECTRON_RUN_AS_NODE: '' }
    });
    process.exit(0);
}

// Since we are now guaranteed to be in Electron, we can get these
const { app, BrowserWindow, ipcMain } = electron;

import { AuthManager } from './auth/AuthManager.ts';
import { StreamAPI } from './api/StreamAPI.ts';
import { 
    WINDOW_CONFIG, 
    PATHS, 
    IPC_CHANNELS, 
    CONSOLE_MESSAGES 
} from './constants.ts';
import { createIpcHandler } from './utils/ipcHandler.ts';

// Main Application Logic
async function init() {
    let mainWindow: electron.BrowserWindow | null = null;
    let streamAPI: StreamAPI | null = null;
    let token: string | null = null;

    async function createWindow() {
        mainWindow = new BrowserWindow({
            ...WINDOW_CONFIG.MAIN,
            webPreferences: {
                preload: path.join(process.cwd(), PATHS.PRELOAD),
                contextIsolation: true,
                nodeIntegration: false,
            },
        });

        const indexPath = path.join(process.cwd(), PATHS.INDEX_HTML);
        if (fs.existsSync(indexPath)) {
            mainWindow.loadFile(indexPath);
        } else {
            console.error('Could not find index.html at', indexPath);
        }

        mainWindow.on('closed', () => {
            mainWindow = null;
        });
    }

    function setupIPC() {
        // Auth handler
        createIpcHandler(IPC_CHANNELS.AUTH_LOGIN, async () => {
            console.log(CONSOLE_MESSAGES.AUTH_START);
            const authManager = new AuthManager();
            token = await authManager.retrieveToken();
            streamAPI = new StreamAPI(token);
            console.log(CONSOLE_MESSAGES.AUTH_SUCCESS);
            return { success: true };
        });

        // Stream handlers with streamAPI check
        createIpcHandler(IPC_CHANNELS.STREAM_INFO, async () => {
            return streamAPI?.getInfo() ?? null;
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });

        createIpcHandler(IPC_CHANNELS.STREAM_SEARCH, async (_: any, query: string) => {
            return streamAPI?.search(query) ?? [];
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });

        createIpcHandler(IPC_CHANNELS.STREAM_START, async (_: any, { title, category }: any) => {
            return streamAPI?.start(title, category) ?? null;
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });

        createIpcHandler(IPC_CHANNELS.STREAM_END, async () => {
            return streamAPI?.end() ?? false;
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });

        createIpcHandler(IPC_CHANNELS.USER_PROFILE, async () => {
            return streamAPI?.getUserProfile() ?? null;
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });

        createIpcHandler(IPC_CHANNELS.STREAM_CURRENT, async () => {
            return streamAPI?.getCurrentStream() ?? null;
        }, { requireStreamApi: true, getStreamApi: () => streamAPI });
    }

    await app.whenReady();
    setupIPC();
    await createWindow();

    app.on('window-all-closed', () => {
        if (process.platform !== 'darwin') app.quit();
    });

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });
}

init().catch(err => {
    console.error('Failed to initialize app:', err);
});
