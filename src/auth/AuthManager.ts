import path from 'path';
import fs from 'fs';
import crypto from 'node:crypto';
import { StreamlabsAuth } from './electron-login.ts';
import { 
    AUTH_CONFIG, 
    PATHS, 
    API_ENDPOINTS, 
    CONSOLE_MESSAGES 
} from '../constants.ts';

export class AuthManager {
    private codeVerifier: string;
    private codeChallenge: string;

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
        const params = new URLSearchParams({
            force_verify: AUTH_CONFIG.FORCE_VERIFY,
            external: AUTH_CONFIG.EXTERNAL,
            skip_splash: AUTH_CONFIG.SKIP_SPLASH,
            tiktok: '1',
            code_challenge: this.codeChallenge,
        });
        return `${API_ENDPOINTS.LOGIN_URL}?${params.toString()}`;
    }

    async retrieveToken(): Promise<string> {
        const tokenPath = path.resolve(process.cwd(), PATHS.TOKENS);

        // Try to load existing token
        if (fs.existsSync(tokenPath)) {
            try {
                const saved = JSON.parse(fs.readFileSync(tokenPath, 'utf-8'));
                if (saved && saved.oauth_token) {
                    console.log(CONSOLE_MESSAGES.AUTH_SAVED_TOKEN);
                    return saved.oauth_token;
                }
            } catch (e) {
                console.error(CONSOLE_MESSAGES.AUTH_LOAD_FAIL, e);
            }
        }

        const authUrl = await this.getAuthUrl();
        const cookiePathAbs = path.resolve(process.cwd(), PATHS.COOKIES);

        console.log(CONSOLE_MESSAGES.AUTH_START_FLOW);

        const auth = new StreamlabsAuth(authUrl, cookiePathAbs, this.codeVerifier);
        const authData = await auth.findToken();

        // Save all data for later discovery scripts
        fs.writeFileSync(tokenPath, JSON.stringify(authData, null, 2));
        console.log(CONSOLE_MESSAGES.AUTH_SAVED);

        return authData.oauth_token;
    }
}
