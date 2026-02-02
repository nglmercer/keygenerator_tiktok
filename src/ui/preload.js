
if (window.__TAURI__) {
    window.tauriAPI = {
        invoke: (command, ...args) => window.__TAURI__.core.invoke(command, ...args),
        listen: (event, callback) => window.__TAURI__.event.listen(event, callback),
        emit: (event, payload) => window.__TAURI__.event.emit(event, payload),
        getCurrentWindow: () => window.__TAURI__.window.getCurrent(),
        path: window.__TAURI__.path,
        shell: window.__TAURI__.shell,
        process: window.__TAURI__.process
    };
} else {
    // Fallback for development without Tauri
    window.tauriAPI = {
        invoke: async (command, ...args) => {
            console.warn('Tauri API not available, command:', command);
            return null;
        },
        listen: (event, callback) => {
            console.warn('Tauri API not available, event:', event);
            return { unsubscribe: () => {} };
        },
        emit: (event, payload) => {
            console.warn('Tauri API not available, event:', event);
        },
        getCurrentWindow: () => null,
        path: {},
        shell: {},
        process: {}
    };
}