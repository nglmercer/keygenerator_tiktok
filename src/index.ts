import path from 'path';
import fs from 'fs';
import { spawn } from 'child_process';
import * as electron from 'electron';

// Self-relaunch in Electron if running in a non-electron environment (like Bun)
if (!process.versions.electron) {
    const electronPath = path.resolve(process.cwd(), 'node_modules', '.bin', 'electron');
    // If we're running the source file with bun, we might need a loader
    // But since we have a build step, it's better to tell the user to use 'bun start'
    // or relaunch with the current file if it's the bundled one.
    const args = process.argv.slice(1);

    // If it's a .ts file, we need to instruct electron how to handle it if we aren't using bun build
    // But since we ARE using bun build, let's just relaunch whatever was called.
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

// Main Application Logic
async function init() {
    let mainWindow: electron.BrowserWindow | null = null;
    let streamAPI: StreamAPI | null = null;
    let token: string | null = null;

    async function createWindow() {
        mainWindow = new BrowserWindow({
            width: 1000,
            height: 800,
            title: 'TikTok Stream Key Generator',
            backgroundColor: '#0f172a',
            webPreferences: {
                preload: path.join(process.cwd(), 'src/ui/preload.js'),
                contextIsolation: true,
                nodeIntegration: false,
            },
        });

        // Try local src path first for development, then fallback to built path if needed
        const indexPath = path.join(process.cwd(), 'src/ui/index.html');
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
        ipcMain.handle('auth:login', async () => {
            try {
                console.log('[Main] Starting authentication flow...');
                const authManager = new AuthManager();
                token = await authManager.retrieveToken();
                streamAPI = new StreamAPI(token);
                console.log('[Main] Authentication successful');
                return { success: true };
            } catch (error: any) {
                console.error('[Main] Authentication failed:', error);
                return { success: false, error: error.message || 'Unknown error during login' };
            }
        });

        ipcMain.handle('stream:info', async () => {
            if (!streamAPI) return null;
            try {
                return await streamAPI.getInfo();
            } catch (error) {
                return null;
            }
        });

        ipcMain.handle('stream:search', async (_event: any, query: string) => {
            if (!streamAPI) return [];
            return await streamAPI.search(query);
        });

        ipcMain.handle('stream:start', async (_event: any, { title, category }: any) => {
            if (!streamAPI) return null;
            return await streamAPI.start(title, category);
        });

        ipcMain.handle('stream:end', async () => {
            if (!streamAPI) return false;
            return await streamAPI.end();
        });
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
