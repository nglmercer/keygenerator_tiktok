export interface WindowUrlContext {
    full: string;
    protocol: string;
    host: string;
    pathname: string;
    hash: string;
    origin: string;
    params: Record<string, string>;
}

export interface WindowDocumentContext {
    title: string;
    referrer: string;
    language: string;
    encoding: string;
}

export interface WindowScreenContext {
    width: number;
    height: number;
    pixelRatio: number;
    orientation: string;
}

export interface WindowContext {
    url: WindowUrlContext;
    document: WindowDocumentContext;
    screen: WindowScreenContext;
    userAgent: string;
    timestamp: string;
}