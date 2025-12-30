import { AuthManager } from './auth/AuthManager';
import { StreamAPI } from './api/StreamAPI';

async function main() {
    console.log('Starting TikTok/Streamlabs Key Generator...');

    try {
        const authManager = new AuthManager();
        const token = await authManager.retrieveToken();
        console.log('Successfully authenticated!');
        console.log('OAuth Token:', token);

        // Example usage: Search for game
        // const api = new StreamAPI(token);
        // const games = await api.search('Fortnite');
        // console.log('Found games:', games);

    } catch (error) {
        console.error('An error occurred:', error);
        process.exit(1);
    }
}

main();
