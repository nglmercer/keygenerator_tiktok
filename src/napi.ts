import { Application } from "webview-napi";
import { safeJsonParse } from "./utils/json.js";
import { IpcMessageRouter } from "./utils/ipc-handler.js";
import { IpcMessageType, type WindowContextPayload, type RawStringPayload } from "./utils/ipc-types.js";
import get_context from "./preload/context.js" with { type: "text" };
import { AuthManager } from "./auth/AuthManager.js";
import { StreamAPI } from "./api/StreamAPI.js";

// TikTok data storage
interface TikTokData {
    uniqueId?: string;
    roomId?: string;
    payload?: any;
    connected: boolean;
}

let tiktokData: TikTokData = {
    connected: false
};

// StreamAPI instance
let streamAPI: StreamAPI | null = null;

const injectionScript = /*js*/`
    (function () {
        window.TiktokPayload = "";
        window.getPayload = function () {
            return window.TiktokPayload;
        };
        const originalSend = WebSocket.prototype.send;
        WebSocket.prototype.send = function (data) {
            if (typeof data === 'string' && data.includes("setUniqueId")) {
                console.log("injectionScript data", data)
                window.TiktokPayload = data;
                window.ipc.postMessage(data);
            }
            return originalSend.apply(this, arguments);
        };
        console.log("💉 Interceptor de WebSocket inyectado");

    })();
`;

async function startWebview() {
  console.log("🔌 Iniciando proceso webview TikFinity...");

  const app = new Application();
  const window = app.createBrowserWindow({
    title: "TikTok Login - Sincronizando TikFinity",
    width: 500,
    height: 700,
  });

  const webview = window.createWebview({
    preload: injectionScript,
    url: "https://google.com/",
    enableDevtools: true,
  });

  webview.openDevtools();

  // Crear el router IPC con opciones configuradas
  const ipcRouter = new IpcMessageRouter({
    enableLogging: true,
    autoGenerateId: true,
    autoTimestamp: true,
    onUnhandledMessage: (msg) => {
      console.warn("[NAPI] Mensaje IPC no manejado:", msg.type);
    },
    onParseError: (error, raw) => {
      console.error("[NAPI] Error parseando mensaje:", error.message);
      console.error("[NAPI] Mensaje crudo:", raw.slice(0, 200));
    },
  });

  // Registrar handlers para los diferentes tipos de mensajes
  ipcRouter
    .on(IpcMessageType.WINDOW_CONTEXT, (payload: WindowContextPayload, message) => {
      console.log("[NAPI] Contexto de ventana recibido:");
      console.log("  - URL:", payload.url.full);
      console.log("  - Título:", payload.document.title);
      console.log("  - Screen:", `${payload.screen.width}x${payload.screen.height}`);
      console.log("  - User Agent:", payload.userAgent.slice(0, 50) + "...");
    })
    .on(IpcMessageType.RAW_STRING, (payload: RawStringPayload, message) => {
      // Este handler captura los mensajes de string crudo (como los de WebSocket)
      console.log("[NAPI] Mensaje de string recibido:");
      console.log("  - Data:", payload.data.slice(0, 200));
      console.log("  - Recibido en:", new Date(payload.receivedAt).toISOString());

      // Aquí puedes procesar el payload de TikTok
      if (payload.data.includes("setUniqueId")) {
        console.log("[NAPI] 🎯 Payload de TikTok detectado!");
        // Procesar el payload de TikTok aquí
        const result = safeJsonParse<any>(payload.data);
        if (result.success) {
          console.log("[NAPI] Payload parseado:", result.data);
          
          // Store TikTok data
          tiktokData.payload = result.data;
          tiktokData.connected = true;
          
          if (result.data && typeof result.data === 'object') {
            if ('uniqueId' in result.data) {
              tiktokData.uniqueId = result.data.uniqueId;
              console.log("[NAPI] TikTok UniqueID:", tiktokData.uniqueId);
            }
            if ('roomId' in result.data) {
              tiktokData.roomId = result.data.roomId;
              console.log("[NAPI] TikTok RoomID:", tiktokData.roomId);
            }
          }
        }
      }
    })
    .on(IpcMessageType.USER_ACTION, (payload, message) => {
      console.log("[NAPI] Acción de usuario:", payload.action);
    })
    .on(IpcMessageType.LOG_EVENT, (payload, message) => {
      const level = payload.level;
      const logFn = level === 'error' ? console.error : level === 'warn' ? console.warn : console.log;
      logFn(`[NAPI] [${level.toUpperCase()}] ${payload.message}`);
      if (payload.metadata) {
        console.log("  Metadata:", payload.metadata);
      }
    });

  // Manejar mensajes IPC del webview
  webview.onIpcMessage((_e, message) => {
    // El mensaje viene como Buffer, lo convertimos a string
    const payload = message.toString();
    console.log("[NAPI] Mensaje raw recibido desde el navegador");

    // Usar el router para procesar el mensaje
    ipcRouter.handle(payload);
  });

  app.onEvent((_e, event) => {
    console.log("[NAPI] Evento de aplicación:", event);
  });

  // Ejecutar script de contexto cada 5 segundos
  setInterval(function(){
    webview.evaluateScript(/*js*/`
      ${get_context}
    `);
  }, 5000);

  const poll = () => {
    if (app.runIteration()) {
      window.id;
      webview.id;
      setTimeout(poll, 10);
    } else {
      process.exit(0);
    }
  };
  poll();
}

/**
 * Perform login and get authentication token
 * Note: This function requires an EventLoop to be passed in for the auth webview
 */
export async function login(eventLoop?: any): Promise<{ success: boolean; token?: string; error?: string }> {
  try {
    console.log("[NAPI] Iniciando proceso de login...");
    const authManager = new AuthManager();
    
    // If no eventLoop is provided, we can't open the auth window
    if (!eventLoop) {
      return { success: false, error: "EventLoop is required for authentication" };
    }
    
    const token = await authManager.retrieveToken(eventLoop);
    
    if (token) {
      // Initialize StreamAPI with the token
      streamAPI = new StreamAPI(token);
      console.log("[NAPI] Login exitoso, token obtenido");
      return { success: true, token };
    } else {
      return { success: false, error: "No se pudo obtener el token" };
    }
  } catch (error: any) {
    console.error("[NAPI] Error en login:", error.message);
    return { success: false, error: error.message };
  }
}

/**
 * Get current TikTok data
 */
export function getTikTokData(): TikTokData {
  return { ...tiktokData };
}

/**
 * Get StreamAPI instance (if logged in)
 */
export function getStreamAPI(): StreamAPI | null {
  return streamAPI;
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
  return streamAPI !== null;
}

/**
 * Check if TikTok connection is established
 */
export function isTikTokConnected(): boolean {
  return tiktokData.connected;
}

// Start the webview if this file is run directly
if (import.meta.main) {
  startWebview();
}

export { startWebview };
