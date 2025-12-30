import { spawn } from 'child_process';
import path from 'path';
import fs from 'fs';
import axios from 'axios';
import crypto from 'node:crypto';

export class AuthManager {
    private CLIENT_KEY = 'awdjaq9ide8ofrtz';
    private REDIRECT_URI = 'https://streamlabs.com/tiktok/auth';
    private AUTH_DATA_URL = 'https://streamlabs.com/api/v5/slobs/auth/data';

    private codeVerifier: string;
    private codeChallenge: string;
    private COOKIES_PATH = 'cookies.json';

    constructor() {
        this.codeVerifier = this.generateCodeVerifier();
        this.codeChallenge = this.generateCodeChallenge(this.codeVerifier);
    }

    private generateCodeVerifier(): string {
        return crypto.randomBytes(64).toString('hex');
    }

    private generateCodeChallenge(verifier: string): string {
        const hash = crypto.createHash('sha256').update(verifier).digest();
        return hash.toString('base64')
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }

    async getAuthUrl(): Promise<string> {
        return `https://streamlabs.com/m/login?force_verify=1&external=mobile&skip_splash=1&tiktok&code_challenge=${this.codeChallenge}`;
    }

    async retrieveToken(): Promise<string> {
        const authUrl = await this.getAuthUrl();
        console.log('Launching Electron to handle authentication...');

        // 1. Compile the Electron script
        try {
            console.log('Compiling Electron script...');
            const { execSync } = await import('child_process');
            execSync('bun build src/auth/electron-login.ts --outfile dist/auth/electron-login.js --target node --external electron', { stdio: 'inherit' });

            // 2. Copy preload.js
            fs.mkdirSync('dist/auth', { recursive: true });
            fs.copyFileSync('src/auth/preload.js', 'dist/auth/preload.js');
        } catch (e) {
            console.error('Failed to compile Electron script:', e);
            throw new Error('Build failed');
        }

        return new Promise((resolve, reject) => {
            const electronPath = path.resolve(process.cwd(), 'node_modules', '.bin', 'electron');
            const scriptPath = path.resolve(process.cwd(), 'dist/auth/electron-login.js');
            const cookiePathAbs = path.resolve(process.cwd(), this.COOKIES_PATH);

            // No ts-node args needed now
            const args = [scriptPath];

            console.log(`Spawning: ${electronPath} ${args.join(' ')}`);

            const child = spawn(electronPath, args, {
                stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
                env: {
                    ...process.env,
                    AUTH_URL: authUrl,
                    COOKIES_PATH: cookiePathAbs,
                    CODE_VERIFIER: this.codeVerifier
                }
            });

            // Timeout mechanism (5 minutes)
            const timeout = setTimeout(() => {
                console.error('[Parent] Auth timed out after 5 minutes.');
                child.kill();
                reject(new Error('Authentication timed out'));
            }, 5 * 60 * 1000);

            if (!child || !child.stdout || !child.stderr) {
                clearTimeout(timeout);
                return reject(new Error('Failed to spawn Electron process'));
            }

            child.stdout.on('data', (data) => {
                // Forward Electron logs
                const output = data.toString().trim();
                console.log(output);
            });

            child.stderr.on('data', (data) => {
                console.error(`[Electron Err]: ${data}`);
            });

            let tokenReceived = false;

            child.on('message', (message: any) => {
                if (message && message.type === 'token-success') {
                    console.log('[Parent] Received token from Electron.');
                    clearTimeout(timeout);
                    tokenReceived = true;
                    resolve(message.token);
                } else if (message && message.type === 'login-success') {
                    console.log('[Parent] Login success signaled (waiting for token...)');
                } else if (message && message.type === 'error') {
                    console.error('[Parent] Error from Electron:', message.error);
                    clearTimeout(timeout);
                    reject(new Error(`Authentication failed: ${message.error}`));
                }
            });

            child.on('close', (code) => {
                clearTimeout(timeout);
                console.log('Electron process closed with code:', code);
                if (!tokenReceived) {
                    reject(new Error('Electron process closed without returning a token'));
                }
            });

            child.on('error', (err) => {
                clearTimeout(timeout);
                reject(new Error(`Failed to start Electron process: ${err.message}`));
            });
        });
    }
}
