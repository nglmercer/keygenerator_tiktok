# TikTok Stream Key Generator

Este proyecto ha sido migrado a TypeScript/JavaScript utilizando Electron para manejar la autenticación de manera robusta.

## Características Actuales

- **Autenticación con Electron**: Utiliza una ventana de navegador real para el login de TikTok y la autorización de Streamlabs, resolviendo problemas de cookies y headers.
- **Persistencia de Sesión**: Guarda y carga cookies automáticamente para evitar logins repetidos.
- **Generación de Claves**: Obtiene automáticamente la clave de transmisión (Stream Key) y la URL RTMP.
- **IPC Logging**: Sistema de logs detallados mediante IPC para depurar la comunicación entre el proceso de renderizado (navegador) y el proceso principal.

## Estructura del Proyecto

- `src/index.ts`: Punto de entrada principal.
- `src/auth/AuthManager.ts`: Gestiona el proceso de autenticación, lanzando el proceso de Electron.
- `src/auth/electron-login.ts`: Script principal de Electron que maneja la ventana, inyección de scripts y recuperación del token.
- `src/auth/preload.ts`: Script de precarga para exponer APIs seguras (IPC) al contexto de la página web.
- `src/api/StreamAPI.ts`: Cliente para interactuar con la API de Streamlabs una vez obtenido el token.

## Uso

1. Instalar dependencias:
   ```bash
   npm install
   ```

2. Ejecutar:
   ```bash
   npm start
   ```

## Detalles Técnicos

### Flujo de Autenticación

1. `AuthManager` genera `code_verifier` y `code_challenge` (PKCE).
2. Se lanza un proceso de Electron (`electron-login.ts`) con las variables de entorno necesarias.
3. Electron carga cookies previas (si existen) y navega a TikTok.
4. El usuario inicia sesión (o se detecta sesión activa).
5. Se navega a la URL de autorización de Streamlabs.
6. Al detectar el redirect de éxito (`success=true`), se inyecta un script en la página.
7. **Inyección y Fetch**: El script inyectado utiliza `fetch` dentro del contexto de la ventana (donde residen las cookies de sesión) para obtener el `oauth_token` desde:
   `https://streamlabs.com/api/v5/slobs/auth/data?code_verifier=...`
8. El resultado se envía de vuelta al proceso principal mediante IPC (`ipcRenderer.send('fetch-result', ...)`).
9. El token se devuelve a `AuthManager` y se guarda la sesión.

### IPC y Debugging

Se ha implementado un puente IPC en `src/auth/preload.js` que permite:
- `window.electronAPI.log(msg)`: Enviar logs desde la consola del navegador a la terminal de node.
- `window.electronAPI.sendResult(channel, data)`: Enviar datos complejos (como el resultado del fetch) al proceso principal.

Esto permite visualizar errores de red (401, 403, etc.) y cuerpos de respuesta directamente en la consola de ejecución.
