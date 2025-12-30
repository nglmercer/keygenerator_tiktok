#!/usr/bin/env bun
import { Command } from 'commander';
import { StreamAPI } from '../api/StreamAPI.js';
import { AuthManager } from '../auth/AuthManager.js';
import { ConfigManager } from '../config/ConfigManager.js';
import process from 'process';

const program = new Command();
const configManager = new ConfigManager();

program
    .name('tiktok-stream-cli')
    .description('CLI to generate Stream Keys for TikTok via Streamlabs')
    .version('1.0.0');

program
    .command('login')
    .description('Login to TikTok via Streamlabs to get an OAuth token')
    .action(async () => {
        try {
            const authManager = new AuthManager();
            const token = await authManager.retrieveToken();
            console.log('Token retrieved successfully!');
            configManager.save({ token });
            console.log('Token saved to config.');
        } catch (error) {
            console.error('Login failed:', error);
            process.exit(1);
        }
    });

program
    .command('info')
    .description('Get current account info')
    .action(async () => {
        const config = configManager.load();
        if (!config.token) {
            console.error('No token found. Please run "login" command first.');
            process.exit(1);
        }
        const api = new StreamAPI(config.token);
        try {
            const info = await api.getInfo();
            console.log(JSON.stringify(info, null, 2));
        } catch (error) {
            console.error('Failed to get info:', error);
        }
    });

program
    .command('search')
    .argument('<game>', 'Game to search for')
    .description('Search for a game category')
    .action(async (game) => {
        const config = configManager.load();
        if (!config.token) {
            console.error('No token found. Please run "login" command first.');
            process.exit(1);
        }
        const api = new StreamAPI(config.token);
        const results = await api.search(game);
        console.log(JSON.stringify(results, null, 2));
    });

program
    .command('start')
    .argument('<game>', 'Game category name or ID')
    .option('-t, --title <title>', 'Stream title', 'TikTok Stream')
    .option('-a, --audience <type>', 'Audience type (0=all)', '0')
    .description('Start a stream and get the key')
    .action(async (game, options) => {
        const config = configManager.load();
        if (!config.token) {
            console.error('No token found. Please run "login" command first.');
            process.exit(1);
        }
        const api = new StreamAPI(config.token);

        // First, try to resolve the game to an ID if possible or pass it as is
        // The previous implementation searched first.
        // We'll implemented a smart lookup: if search returns exact match, use it.
        let category = game;
        console.log(`Searching for category: ${game}...`);
        const searchResults = await api.search(game);
        if (searchResults.length > 0) {
            // Try to find exact match
            const match = searchResults.find(c => c.full_name.toLowerCase() === game.toLowerCase());
            if (match) {
                category = match.id;
                console.log(`Found exact match: ${match.full_name} (${match.id})`);
            } else {
                console.log('No exact match found, using first result:', searchResults[0]!.full_name);
                category = searchResults[0]!.id;
            }
        } else {
            console.log('No categories found, using input as ID/Category directly.');
        }

        console.log('Starting stream...');
        const result = await api.start(options.title, category, options.audience);
        if (result) {
            console.log('Stream started successfully!');
            console.log('------------------------------------------------');
            console.log(`Server: ${result.rtmpUrl}`);
            console.log(`Key:    ${result.streamKey}`);
            console.log(`ID:     ${result.id}`);
            console.log('------------------------------------------------');

            // Save to config for "end" command
            // We might want to store the last stream ID
            // But for now, we just output it.
        } else {
            console.error('Failed to start stream.');
        }
    });

program
    .command('end')
    .argument('<id>', 'Stream ID to end')
    .description('End a running stream')
    .action(async (id) => {
        const config = configManager.load();
        if (!config.token) {
            console.error('No token found. Please run "login" command first.');
            process.exit(1);
        }
        const api = new StreamAPI(config.token);
        const success = await api.end(id);
        if (success) {
            console.log('Stream ended successfully.');
        } else {
            console.error('Failed to end stream.');
        }
    });

program.parse(process.argv);
