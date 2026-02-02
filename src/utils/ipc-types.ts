/**
 * Tipos de mensajes IPC soportados
 */
export enum IpcMessageType {
    WINDOW_CONTEXT = 'WINDOW_CONTEXT',
    USER_ACTION = 'USER_ACTION',
    LOG_EVENT = 'LOG_EVENT',
    RAW_STRING = 'RAW_STRING',  // Para mensajes de string sin estructura JSON
}

/**
 * Payload para el contexto de ventana
 */
export interface WindowContextPayload {
    url: {
        full: string;
        protocol: string;
        host: string;
        pathname: string;
        hash: string;
        origin: string;
        params: Record<string, string>;
    };
    document: {
        title: string;
        referrer: string;
        language: string;
        encoding: string;
    };
    screen: {
        width: number;
        height: number;
        pixelRatio: number;
        orientation: string;
    };
    userAgent: string;
    timestamp: string;
}

/**
 * Payload para acciones de usuario
 */
export interface UserActionPayload {
    action: string;
    timestamp: number;
}

/**
 * Payload para eventos de log
 */
export interface LogEventPayload {
    level: 'info' | 'warn' | 'error' | 'debug';
    message: string;
    metadata?: Record<string, unknown>;
}

/**
 * Payload para mensajes de string crudo
 */
export interface RawStringPayload {
    data: string;
    receivedAt: number;
}

/**
 * Estructura base de mensaje IPC
 */
export interface IpcMessageBase {
    type: IpcMessageType;
    payload: unknown;
    timestamp?: number;
    id?: string;
}

/**
 * Mensajes IPC tipados usando discriminación de tipos
 */
export type IpcMessage =
    | { type: IpcMessageType.WINDOW_CONTEXT; payload: WindowContextPayload; timestamp?: number; id?: string }
    | { type: IpcMessageType.USER_ACTION; payload: UserActionPayload; timestamp?: number; id?: string }
    | { type: IpcMessageType.LOG_EVENT; payload: LogEventPayload; timestamp?: number; id?: string }
    | { type: IpcMessageType.RAW_STRING; payload: RawStringPayload; timestamp?: number; id?: string };

/**
 * Extractor de payload para un tipo específico
 */
export type IpcPayload<T extends IpcMessageType> = Extract<IpcMessage, { type: T }>['payload'];

/**
 * Handler function type para un tipo específico de mensaje
 */
export type IpcHandler<T extends IpcMessageType> = (payload: IpcPayload<T>, message: IpcMessage) => void;

/**
 * Mapa de handlers para cada tipo de mensaje
 */
export type IpcHandlerMap = {
    [K in IpcMessageType]?: IpcHandler<K>;
};

/**
 * Resultado del parsing de un mensaje IPC
 */
export type ParseResult<T> =
    | { success: true; data: T }
    | { success: false; error: Error; rawMessage?: string };

/**
 * Opciones de configuración para el IPC handler
 */
export interface IpcHandlerOptions {
    /** Habilitar logging de mensajes recibidos */
    enableLogging?: boolean;
    /** Callback para mensajes no manejados */
    onUnhandledMessage?: (message: IpcMessage) => void;
    /** Callback para errores de parsing */
    onParseError?: (error: Error, rawMessage: string) => void;
    /** Generar IDs automáticamente */
    autoGenerateId?: boolean;
    /** Añadir timestamp automáticamente si no existe */
    autoTimestamp?: boolean;
}
