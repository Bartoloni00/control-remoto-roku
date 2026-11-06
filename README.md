# Control Remoto Roku

Aplicación de escritorio nativa para controlar dispositivos Roku a través de la red local, desarrollada en Rust utilizando `eframe` y `egui`. Cuenta con una interfaz gráfica moderna, un pad direccional personalizado dibujado por código y soporte para accesos directos a aplicaciones de streaming.

---

## Características Principales

- **Pad Direccional Personalizado:** Renderizado mediante el `painter` de `egui`, con botones en cruz y un botón central `OK` con un degradado dinámico (naranja → violeta → azul) en forma de trébol.
- **Control Real (Fire-and-forget):** Envía eventos de presionado (`/keydown`) y liberación (`/keyup`) para las flechas direccionales, permitiendo mantener presionado el botón como en un control físico real.
- **Detección Automática de Aplicaciones:** Al iniciar, consulta la API del Roku (`/query/apps`) para mapear dinámicamente los IDs de las aplicaciones instaladas (adaptándose a cambios regionales, por ejemplo, HBO/Max o Spotify).
- **Gestión de Configuración:** Guarda automáticamente la última IP utilizada en `~/.config/control-remoto-roku/config.toml`.
- **Soporte de Teclado:** Se puede operar completamente desde el teclado:
  - Flechas: Navegación.
  - `Enter`: OK.
  - `Escape` o `H`: Home.
- **Diseño No Bloqueante:** Todas las peticiones HTTP se ejecutan en segundo plano utilizando hilos independientes y `ureq`, evitando que la interfaz se congele si el dispositivo no responde.

---

## Protocolo y API

La aplicación se comunica mediante el protocolo HTTP estándar de Roku sobre el puerto `8060` (IP por defecto: `192.168.0.38`):

- **Pad y Teclas:** 
  - Movimiento: `POST /keydown/[Up|Down|Left|Right]` y `POST /keyup/...`
  - Selección: `POST /keypress/Select`
  - Home: `POST /keypress/Home`
- **Lanzamiento de Apps (`POST /launch/{id}`):**
  - **Netflix:** ID `12`
  - **YouTube:** ID `837`
  - **HBO / Max:** ID `61322` (con respaldo automático)
  - **Spotify:** ID de respaldo `22297`
- **Estado y Diagnóstico:** 
  - Consulta el nombre del dispositivo mediante `GET /query/device-info`.
  - Muestra alertas contextuales si el Roku está apagado, la IP es incorrecta o si la opción *"Control by mobile apps"* está desactivada en el Roku (*Ajustes > Sistema > Ajustes avanzados*).

---

## Estructura del Proyecto

El crate está organizado de la siguiente manera:

- `Cargo.toml`: Dependencias principales (`eframe`, `egui`, `ureq`, etc.).
- `src/main.rs`: Arranque de la interfaz con `eframe` y lógica de carga/guardado de la configuración en disco.
- `src/roku.rs`: Lógica de red, manejo de IPs, teclas, IDs de aplicaciones y llamadas HTTP.
- `src/ui.rs`: Estado de la ventana, renderizado del pad vectorial, accesos directos y atajos de teclado.

---

## Requisitos Previos

1. Tener instalado [Rust y Cargo](https://rustup.rs/).
2. Asegurarte de que tu dispositivo Roku esté encendido y en la misma red local que tu PC.
3. En tu Roku, verifica que esté habilitado el control por aplicaciones móviles (*Ajustes > Sistema > Ajustes avanzados > Control por aplicaciones móviles* configurado en **Por lo general, permitir**).

---

## Compilación y Ejecución

1. Clona o sitúate en la carpeta raíz del proyecto.
2. Compila y ejecuta la aplicación en modo desarrollo con Cargo:
   ```bash
   cargo run