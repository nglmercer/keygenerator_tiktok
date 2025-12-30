import path from 'path';
import crypto from 'node:crypto';
import { StreamlabsAuth } from './electron-login.ts';

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
        const cookiePathAbs = path.resolve(process.cwd(), this.COOKIES_PATH);

        console.log('[AuthManager] Starting authentication via internal Electron window...');

        const auth = new StreamlabsAuth(authUrl, cookiePathAbs, this.codeVerifier);
        return await auth.findToken();
    }
}
