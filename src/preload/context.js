const emitWindowContext = () => {
    const context = {
        type: 'WINDOW_CONTEXT',
        payload: {
            url: {
                full: window.location.href,
                protocol: window.location.protocol,
                host: window.location.host,
                pathname: window.location.pathname,
                hash: window.location.hash,
                origin: window.location.origin,
                params: Object.fromEntries(new URLSearchParams(window.location.search))
            },
            document: {
                title: document.title,
                referrer: document.referrer,
                language: navigator.language,
                encoding: document.characterSet
            },
            screen: {
                width: window.innerWidth,
                height: window.innerHeight,
                pixelRatio: window.devicePixelRatio,
                orientation: screen.orientation ? screen.orientation.type : 'unknown'
            },
            userAgent: navigator.userAgent,
            timestamp: new Date().toISOString()
        }
    };

    const jsonPayload = JSON.stringify(context);

    console.log({context});

    if (window.ipc && typeof window.ipc.postMessage === 'function') {
        window.ipc.postMessage(jsonPayload);
    } else {
        console.warn("window.ipc.postMessage, not exist", context);
    }
};
emitWindowContext();
