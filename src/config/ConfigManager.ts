import fs from 'fs';
import path from 'path';

export interface AppConfig {
    token?: string;
    title?: string;
    game?: string;
    audienceType?: string;
    suppressDonationReminder?: boolean;
}

export class ConfigManager {
    private configPath: string;
    private config: AppConfig;

    constructor(filename: string = 'config.json') {
        this.configPath = path.resolve(process.cwd(), filename);
        this.config = {
            audienceType: '0',
            suppressDonationReminder: false
        };
    }

    load(): AppConfig {
        if (fs.existsSync(this.configPath)) {
            try {
                const data = fs.readFileSync(this.configPath, 'utf-8');
                this.config = { ...this.config, ...JSON.parse(data) };
            } catch (error) {
                console.error('Error loading config:', error);
            }
        }
        return this.config;
    }

    save(newConfig: Partial<AppConfig>): void {
        this.config = { ...this.config, ...newConfig };
        try {
            fs.writeFileSync(this.configPath, JSON.stringify(this.config, null, 2));
        } catch (error) {
            console.error('Error saving config:', error);
        }
    }

    get(key: keyof AppConfig): any {
        return this.config[key];
    }
}
