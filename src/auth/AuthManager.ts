import { firefox } from 'playwright';
import axios from 'axios';
import crypto from 'node:crypto';

export class AuthManager {
    private CLIENT_KEY = 'awdjaq9ide8ofrtz';
    private REDIRECT_URI = 'https://streamlabs.com/tiktok/auth';
    // Note: The migration guide mentioned a different AUTH_DATA endpoint, but the Python code used this one.
    // We will stick to the Python implementation's logic regarding endpoints where possible or the guide if clearer.
    // The guide says: https://streamlabs.com/api/v5/slobs/auth/data
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
        console.log('Opening browser for authentication...');

        // Launch playwright browser (headful so user can interact)
        const browser = await firefox.launch({ headless: false });
        const context = await browser.newContext();

        // Load cookies if they exist
        await this.loadCookies(context);

        const page = await context.newPage();

        let oauthToken: string | null = null;

        try {
            // Step 1: Login to TikTok
            console.log('Navigating to TikTok login page...');
            await page.goto('https://www.tiktok.com/login');

            console.log('Waiting for user to log in (waiting for profile icon)...');
            // Wait indefinitely for the user to log in. 
            // usage of data-e2e="profile-icon" is a common stable selector for TikTok web
            await page.waitForSelector('[data-e2e="profile-icon"]', { timeout: 0 });
            console.log('Login detected/verified.');

            // Step 2: Navigate to Streamlabs Auth
            console.log('Proceeding to Streamlabs OAuth flow...');
            await page.goto(authUrl);

            // Wait for redirect to something that indicates success or the redirect URI
            // The Python code waited for "success=true" in url.
            // And then made a separate request to get the token.
            // Wait up to 600s (10 min) like the python script
            await page.waitForURL((url: URL) => {
                return url.toString().includes('success=true');
            }, { timeout: 600000 });

            console.log('Authentication successful in browser. Retrieving token...');

            // Save cookies after successful login
            await this.saveCookies(context);

            // After success=true, we request the token using the code_verifier
            const params = {
                code_verifier: this.codeVerifier
            };

            const response = await axios.get(this.AUTH_DATA_URL, { params });

            if (response.data && response.data.success) {
                oauthToken = response.data.data.oauth_token;
            } else {
                throw new Error(`Failed to retrieve token from API: ${JSON.stringify(response.data)}`);
            }

        } catch (error) {
            console.error('Authentication failed:', error);
            throw error;
        } finally {
            await browser.close();
        }

        if (!oauthToken) {
            throw new Error('OAuth token was not retrieved.');
        }

        return oauthToken;
    }

    private async loadCookies(context: any) {
        try {
            const fs = await import('fs');
            const path = await import('path');
            const cookiePath = path.resolve(process.cwd(), this.COOKIES_PATH);

            if (fs.existsSync(cookiePath)) {
                const cookies = JSON.parse(fs.readFileSync(cookiePath, 'utf-8'));
                await context.addCookies(cookies);
                console.log('Cookies loaded.');
            }
        } catch (error) {
            console.warn('Failed to load cookies:', error);
        }
    }

    private async saveCookies(context: any) {
        try {
            const fs = await import('fs');
            const path = await import('path');
            const cookiePath = path.resolve(process.cwd(), this.COOKIES_PATH);

            const cookies = await context.cookies();
            fs.writeFileSync(cookiePath, JSON.stringify(cookies, null, 2));
            console.log('Cookies saved.');
        } catch (error) {
            console.warn('Failed to save cookies:', error);
        }
    }
}
