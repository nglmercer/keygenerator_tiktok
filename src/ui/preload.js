const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    invoke: (channel, ...args) => ipcRenderer.invoke(channel, ...args),
    on: (channel, callback) => ipcRenderer.on(channel, (event, ...args) => callback(...args)),
    send: (channel, ...args) => ipcRenderer.send(channel, ...args)
});
