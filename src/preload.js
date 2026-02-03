const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
    log: (msg) => {
        ipcRenderer.send('log-console', msg)
        console.log(msg)
    },
    sendResult: (channel, data) => ipcRenderer.send(channel, data)
});
