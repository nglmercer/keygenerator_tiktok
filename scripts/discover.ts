import axios from 'axios';
import fs from 'fs';
import path from 'path';

async function discover() {
    const tokenPath = path.resolve(process.cwd(), 'tokens.json');
    if (!fs.existsSync(tokenPath)) {
        console.error('Error: tokens.json not found. Please run the app and login first.');
        process.exit(1);
    }

    const authData = JSON.parse(fs.readFileSync(tokenPath, 'utf-8'));
    const token = authData.oauth_token;

    console.log(`Using OAuth Token: ${token}`);

    const baseApi = 'https://streamlabs.com/api/v5/slobs';
    const headers = {
        'Authorization': `Bearer ${token}`,
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) StreamlabsDesktop/1.17.0 Chrome/122.0.6261.156 Electron/29.3.1 Safari/537.36'
    };

    const namespaces = ['tiktok', 'auth', 'user', 'social-accounts', 'integration'];
    const paths = [
        '/info', '/info?category=gaming', '/stream/current', '/stream/status',
        '/user', '/user/profile', '/settings', '/categories', '/config'
    ];

    console.log('\n--- Deep Discovery Mode ---');
    for (const ns of namespaces) {
        for (const p of paths) {
            const url = `${baseApi}/${ns}${p}`;
            try {
                const res = await axios.get(url, { headers });
                console.log(`[PASS] GET ${url} -> ${res.status}`);
                const filename = `discovery_${ns}_${p.replace(/[^a-z0-9]/gi, '_')}.json`;
                fs.writeFileSync(filename, JSON.stringify(res.data, null, 2));
            } catch (e: any) {
                if (e.response?.status && e.response.status !== 404) {
                    // console.log(`[HIT] ${url} -> ${e.response.status}`);
                }
            }
        }
    }

    // Specially look for categories
    console.log('\n--- Searching for Categories ---');
    const categoriesToTry = ['gaming', 'music', 'chatting', 'creative', 'other'];
    for (const cat of categoriesToTry) {
        try {
            const url = `${baseApi}/tiktok/info?category=${cat}`;
            const res = await axios.get(url, { headers });
            if (res.data.categories) {
                console.log(`[INFO] Found ${res.data.categories.length} categories for search term: ${cat}`);
            }
        } catch (e) { }
    }
}

discover().catch(console.error);
