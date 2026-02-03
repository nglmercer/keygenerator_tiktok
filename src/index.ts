import { spawn } from 'child_process';
import * as electron from 'electron';
import path from 'path';

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

const { app } = electron;

// Disable GPU hardware acceleration to fix VAAPI/libva errors on Linux
app.commandLine.appendSwitch('ignore-gpu-blocklist');
app.commandLine.appendSwitch('disable-gpu-driver-bug-workarounds');
app.disableHardwareAcceleration();

import { AuthManager } from './auth/AuthManager';
import { StreamAPI } from './api/StreamAPI';
import { IPC_CHANNELS, CONSOLE_MESSAGES } from './constants';
import { createIpcHandler } from './utils/ipcHandler';
import { MainWindowManager } from './utils/windowManager';
import { FileUtils } from './utils/fileUtils';

// Main Application Logic
async function init() {
    let streamAPI: StreamAPI | null = null;
    let token: string | null = null;

    const mainWindow = new MainWindowManager();

    function setupIPC() {
        createIpcHandler(IPC_CHANNELS.AUTH_LOGIN, async () => {
            console.log(CONSOLE_MESSAGES.AUTH_START);
            const authManager = new AuthManager();
            token = await authManager.retrieveToken();
            streamAPI = new StreamAPI(token);
            console.log(CONSOLE_MESSAGES.AUTH_SUCCESS);
            return { success: true };
        });

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
    mainWindow.create();
    mainWindow.load();

    app.on('window-all-closed', () => {
        if (process.platform !== 'darwin') app.quit();
    });

    app.on('activate', () => {
        if (!mainWindow.exists()) {
            mainWindow.create();
            mainWindow.load();
        }
    });
}

init().catch(err => {
    console.error('Failed to initialize app:', err);
});
