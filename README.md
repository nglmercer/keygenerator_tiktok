# TikTok Stream Key Generator (GUI Version)

Este proyecto es una herramienta completa para generar claves de transmisión de TikTok utilizando la API de Streamlabs. Cuenta con una interfaz gráfica (GUI) moderna basada en Electron.

## Características

-   **Interfaz Gráfica Premium**: Dashboard moderno con modo oscuro y feedback en tiempo real.
-   **Autenticación Integrada**: Maneja el login de TikTok y Streamlabs automáticamente.
-   **Gestión de Streams**:
    -   Búsqueda de categorías/juegos (con truncado inteligente para evitar errores de la API).
    -   Inicio y parada de transmisiones.
    -   Visualización y copia rápida de URL RTMP y Stream Key.
-   **Persistencia de Sesión**: Guarda cookies automáticamente para sesiones futuras.
-   **Comunicación IPC**: Arquitectura robusta que separa la lógica de la API de la interfaz de usuario.

## Estructura del Proyecto

-   `src/index.ts`: Proceso principal de Electron y manejador de IPC.
-   `src/ui/`: Archivos de la interfaz de usuario (HTML, CSS, JS, Preload).
-   `src/api/StreamAPI.ts`: Implementación de las APIs de Streamlabs (Start, End, Search, Info).
-   `src/auth/AuthManager.ts`: Orquestador de la autenticación PKCE.
-   `src/auth/electron-login.ts`: Ventana dedicada para el proceso de login.

## Uso

1.  **Instalar dependencias**:
    ```bash
    bun install
    ```

2.  **Iniciar la aplicación**:
    ```bash
    # Usando electron con ts-node para ejecutar directamente
    npx electron -r ts-node/register src/index.ts
    ```

## Desarrollo

Para ejecutar el proyecto en modo desarrollo con Bun:

```bash
# Ejecutar directamente con electron
electron .
```

*Nota: Asegúrate de tener las dependencias instaladas y de que el entry point apunte correctamente.*

## Créditos

Basado en la lógica de [Loukious/StreamLabsTikTokStreamKeyGenerator](https://github.com/Loukious/StreamLabsTikTokStreamKeyGenerator/).
