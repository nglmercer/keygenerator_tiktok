/**
 * Clase para manejar mensajes IPC con soporte para strings y objetos JSON
 */
import { type IpcMessage, IpcMessageType, type IpcHandlerMap, type IpcHandlerOptions } from './ipc-types.js';
import { safeJsonParse } from './json.js';

export class IpcMessageRouter {
    private handlers: IpcHandlerMap = {};
    private options: Required<IpcHandlerOptions>;

    private static readonly defaultOptions: Required<IpcHandlerOptions> = {
        enableLogging: false,
        onUnhandledMessage: () => {},
        onParseError: (error) => console.error('Error parsing IPC message:', error),
        autoGenerateId: true,
        autoTimestamp: true,
    };

    constructor(options: IpcHandlerOptions = {}) {
        this.options = { ...IpcMessageRouter.defaultOptions, ...options };
    }

    /**
     * Registra un handler para un tipo específico de mensaje
     */
    on<T extends IpcMessageType>(
        type: T,
        handler: IpcHandlerMap[T]
    ): this {
        this.handlers[type] = handler;
        return this;
    }

    /**
     * Registra múltiples handlers a la vez
     */
    registerHandlers(handlers: IpcHandlerMap): this {
        this.handlers = { ...this.handlers, ...handlers };
        return this;
    }

    /**
     * Elimina un handler para un tipo específico
     */
    off<T extends IpcMessageType>(type: T): this {
        delete this.handlers[type];
        return this;
    }

    /**
     * Procesa un mensaje crudo (string o Buffer)
     * Intenta parsear como JSON primero, si falla lo trata como string crudo
     */
    handle(rawMessage: string | Buffer): void {
        const messageStr = Buffer.isBuffer(rawMessage) ? rawMessage.toString('utf-8') : rawMessage;

        if (this.options.enableLogging) {
            console.log(`[IPC] Received: ${messageStr.slice(0, 200)}...`);
        }

        // Intentar parsear como JSON estructurado
        const jsonResult = safeJsonParse<IpcMessage>(messageStr);

        if (jsonResult.success) {
            const message = this.enrichMessage(jsonResult.data);
            this.processMessage(message);
            return;
        }

        // Si falla el parsing JSON, tratar como string crudo
        this.handleRawString(messageStr);
    }

    /**
     * Procesa múltiples mensajes delimitados (newline-delimited JSON, etc.)
     */
    handleMultiple(rawMessages: string, delimiter: string = '\n'): void {
        const messages = rawMessages.split(delimiter).filter(m => m.trim().length > 0);
        for (const msg of messages) {
            this.handle(msg);
        }
    }

    /**
     * Valida si un mensaje tiene la estructura correcta de IPC
     */
    private isValidIpcMessage(message: unknown): message is IpcMessage {
        if (!message || typeof message !== 'object') {
            return false;
        }

        const msg = message as Record<string, unknown>;

        // Debe tener un campo 'type' que sea un string válido de IpcMessageType
        if (!msg.type || typeof msg.type !== 'string') {
            return false;
        }

        // Verificar que el type sea un valor válido del enum
        if (!Object.values(IpcMessageType).includes(msg.type as IpcMessageType)) {
            return false;
        }

        // Debe tener un payload (puede ser cualquier tipo)
        if (!('payload' in msg)) {
            return false;
        }

        return true;
    }

    /**
     * Enriquece el mensaje con metadata adicional
     */
    private enrichMessage(message: IpcMessage): IpcMessage {
        const enriched = { ...message };

        if (this.options.autoTimestamp && !enriched.timestamp) {
            enriched.timestamp = Date.now();
        }

        if (this.options.autoGenerateId && !enriched.id) {
            enriched.id = this.generateId();
        }

        return enriched;
    }

    /**
     * Genera un ID único para el mensaje
     */
    private generateId(): string {
        return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
    }

    /**
     * Procesa un mensaje IPC válido
     */
    private processMessage(message: IpcMessage): void {
        if (!this.isValidIpcMessage(message)) {
            this.options.onParseError(
                new Error('Invalid IPC message structure'),
                JSON.stringify(message)
            );
            return;
        }

        const handler = this.handlers[message.type];

        if (!handler) {
            if (this.options.enableLogging) {
                console.warn(`[IPC] No handler registered for type: ${message.type}`);
            }
            this.options.onUnhandledMessage(message);
            return;
        }

        try {
            // Llamar al handler con el payload y el mensaje completo
            (handler as (payload: unknown, message: IpcMessage) => void)(
                message.payload,
                message
            );

            if (this.options.enableLogging) {
                console.log(`[IPC] Handled type: ${message.type}`);
            }
        } catch (error) {
            console.error(`[IPC] Error handling message type ${message.type}:`, error);
        }
    }

    /**
     * Maneja strings crudos que no son JSON válido
     */
    private handleRawString(data: string): void {
        const rawMessage: IpcMessage = {
            type: IpcMessageType.RAW_STRING,
            payload: {
                data,
                receivedAt: Date.now(),
            },
            timestamp: Date.now(),
            id: this.generateId(),
        };

        this.processMessage(rawMessage);
    }

    /**
     * Crea un mensaje IPC válido a partir de datos
     */
    static createMessage<T extends IpcMessageType>(
        type: T,
        payload: Extract<IpcMessage, { type: T }>['payload']
    ): IpcMessage {
        return {
            type,
            payload,
            timestamp: Date.now(),
            id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
        } as IpcMessage;
    }
}

/**
 * Función de utilidad para compatibilidad hacia atrás
 * Mantiene la interfaz simple del handler original
 */
export function createIpcHandler(options: IpcHandlerOptions = {}) {
    return new IpcMessageRouter(options);
}

/**
 * Función legacy para compatibilidad (mantiene la firma original)
 */
export function handleIpcMessage(
    rawMessage: string,
    handlers: IpcHandlerMap,
    options: IpcHandlerOptions = {}
): void {
    const router = new IpcMessageRouter(options);
    router.registerHandlers(handlers);
    router.handle(rawMessage);
}
