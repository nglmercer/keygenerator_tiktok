# TikTok Stream Key Generator

Electron application for generating TikTok stream keys via Streamlabs API integration.

## 📋 Table of Contents

- [Features](#features)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [Development](#development)
- [API Reference](#api-reference)
- [Utilities](#utilities)
- [Configuration](#configuration)

## ✨ Features

- 🔐 OAuth authentication with TikTok/Streamlabs
- 🎥 Stream key generation
- 🔍 Category search for streams
- 👤 User profile management
- 🖥️ Electron-based desktop application

## 🏗️ Architecture

The project follows a modular architecture with clear separation of concerns:

```
src/
├── constants.ts          # Centralized magic strings and config
├── index.ts              # Application entry point
├── api/                  # API layer
│   └── StreamAPI.ts      # Streamlabs TikTok API client
├── auth/                 # Authentication layer
│   ├── AuthManager.ts    # Token management
│   └── electron-login.ts # In-app browser login
└── utils/                # Shared utilities
    ├── apiClient.ts      # Base API client
    ├── fileUtils.ts      # File operations
    ├── ipcHandler.ts     # IPC utilities
    └── windowManager.ts  # Window lifecycle
```

### Design Patterns Used

- **Singleton Pattern**: `FileUtils`, `TokenStorage`
- **Factory Pattern**: `createIpcHandler()`, `createWindowWithHandlers()`
- **Inheritance**: `StreamAPI` extends `BaseApiClient`
- **Composition**: `AuthWindowManager` extends `WindowManager`

## 🚀 Getting Started

### Prerequisites

- [Bun](https://bun.sh/) runtime
- [Node.js](https://nodejs.org/) (for Electron)
- [TypeScript](https://www.typescriptlang.org/)

### Installation

```bash
# Install dependencies
bun install

# Build the application
bun run build

# Start in development mode
bun run start
```

### Available Scripts

| Script | Description |
|--------|-------------|
| `bun run build` | Build TypeScript and bundle Electron app |
| `bun run build:code` | Bundle TypeScript code only |
| `bun run typecheck` | Run TypeScript type checking |
| `bun run start` | Build and start Electron app |
| `bun run discover` | Run discovery script |

## 💻 Development

### Project Structure

```
keygenerator_tiktok/
├── package.json          # Dependencies and scripts
├── tsconfig.json         # TypeScript configuration
├── src/
│   ├── constants.ts      # Magic strings & config values
│   ├── index.ts          # Main entry point
│   ├── api/
│   │   └── StreamAPI.ts  # API client for Streamlabs
│   ├── auth/
│   │   ├── AuthManager.ts      # Token storage & retrieval
│   │   └── electron-login.ts   # OAuth login window
│   └── utils/
│       ├── apiClient.ts        # BaseApiClient class
│       ├── fileUtils.ts        # File I/O utilities
│       ├── ipcHandler.ts       # IPC handler factory
│       └── windowManager.ts    # Window lifecycle manager
└── tests/
    ├── api.test.ts       # API tests
    └── auth.test.ts      # Auth tests
```

### Adding New API Endpoints

1. **Add constants** in `src/constants.ts`:
   ```typescript
   export const API_ENDPOINTS = {
       // ... existing endpoints
       NEW_ENDPOINT: `${API_BASE_URL}/new/path`,
   };
   ```

2. **Extend StreamAPI** in `src/api/StreamAPI.ts`:
   ```typescript
   async newMethod(): Promise<ReturnType> {
       return await this.get('/new-endpoint');
   }
   ```

3. **Add IPC handler** in `src/index.ts`:
   ```typescript
   createIpcHandler(IPC_CHANNELS.NEW_CHANNEL, async () => {
       return streamAPI?.newMethod();
   });
   ```

### Adding New Utilities

Utilities should be placed in `src/utils/` and follow these guidelines:

1. **Single Responsibility**: Each utility should do one thing well
2. **Type Safety**: Export TypeScript types
3. **Error Handling**: Handle errors gracefully with logging
4. **Documentation**: Add JSDoc comments

Example utility structure:
```typescript
// src/utils/myUtility.ts
import { SomeConfig } from '../constants';

/**
 * Description of what the utility does
 */
export function myUtility(input: string): string {
    // Implementation
}

/**
 * Class-based utility for stateful operations
 */
export class MyUtilityClass {
    constructor(private config: SomeConfig) {}
    
    public doSomething(): void {
        // Implementation
    }
}
```

## 📖 API Reference

### StreamAPI

```typescript
import { StreamAPI } from './api/StreamAPI';

const api = new StreamAPI(token);

// Search categories
const categories = await api.search('gaming');

// Start stream
const streamInfo = await api.start('My Stream', 'category_id');

// End stream
const success = await api.end(streamId);

// Get user profile
const profile = await api.getUserProfile();
```

### BaseApiClient

```typescript
import { BaseApiClient } from './utils/apiClient';

class MyAPI extends BaseApiClient {
    constructor(token: string) {
        super('https://api.example.com', token);
    }

    async myRequest(): Promise<MyData> {
        return await this.get('/endpoint');
    }
}
```

### WindowManager

```typescript
import { WindowManager } from './utils/windowManager';

const window = new WindowManager({
    title: 'My Window',
    width: 800,
    height: 600,
    preloadPath: './preload.js',
});

window.create();
window.loadFile('./index.html');
```

### FileUtils

```typescript
import { FileUtils, TokenStorage, ConfigStorage } from './utils/fileUtils';

// JSON file operations
const data = FileUtils.readJson('config.json', defaultConfig);
FileUtils.writeJson('config.json', data);

// Token storage
const tokens = new TokenStorage('tokens.json');
const token = tokens.get();
tokens.save({ oauth_token: token });
```

### IPC Handlers

```typescript
import { createIpcHandler } from './utils/ipcHandler';

// Create handler with automatic error handling
createIpcHandler('my-channel', async (event, args) => {
    return await doSomething();
});

// Create handler requiring authentication
createIpcHandler('auth-required', async () => {
    return await protectedAction();
}, { requireStreamApi: true, getStreamApi: () => api });
```

## ⚙️ Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ELECTRON_RUN_AS_NODE` | Used for Electron relaunch |

### Constants Configuration

All magic strings are centralized in `src/constants.ts`:

```typescript
// IPC Channels
IPC_CHANNELS.AUTH_LOGIN
IPC_CHANNELS.STREAM_START
IPC_CHANNELS.STREAM_END
// ...

// API Endpoints
API_ENDPOINTS.TIKTOK_BASE
API_ENDPOINTS.AUTH_DATA
// ...

// Window Config
WINDOW_CONFIG.MAIN
WINDOW_CONFIG.AUTH
```

## 🔧 Testing

```bash
# Run all tests
bun test

# Run API tests
bun test -- tests/api.test.ts

# Run auth tests
bun test -- tests/auth.test.ts
```

## 📦 Building

```bash
# Production build
bun run build

# Output: dist/main.js
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run type checking: `bun run typecheck`
5. Run tests: `bun test`
6. Submit a pull request

## 📄 License

Private project - All rights reserved.
