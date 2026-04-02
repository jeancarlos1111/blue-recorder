# Blue Recorder — Guía de Desarrollo

Grabador de escritorio simple para Linux, escrito en **Rust**, con interfaz GTK 4 y soporte para Wayland (vía XDG Desktop Portal / PipeWire) y Xorg (vía ffmpeg + x11grab).

---

## Tabla de contenidos

1. [Arquitectura del proyecto](#1-arquitectura-del-proyecto)
2. [Requisitos del sistema](#2-requisitos-del-sistema)
3. [Dependencias (crates)](#3-dependencias-crates)
4. [Módulos del código fuente](#4-módulos-del-código-fuente)
5. [Cómo compilar](#5-cómo-compilar)
6. [Cómo ejecutar en desarrollo](#6-cómo-ejecutar-en-desarrollo)
7. [Variables de entorno útiles](#7-variables-de-entorno-útiles)
8. [Flujo de grabación](#8-flujo-de-grabación)
9. [Configuración persistente](#9-configuración-persistente)
10. [Internacionalización (i18n)](#10-internacionalización-i18n)
11. [Empaquetado](#11-empaquetado)
    - [Snap](#snap)
    - [Flatpak](#flatpak)
12. [Distribución](#12-distribución)
13. [Cómo mejorar el proyecto](#13-cómo-mejorar-el-proyecto)
14. [Convenciones de código](#14-convenciones-de-código)

---

## 1. Arquitectura del proyecto

```
blue-recorder/
├── Cargo.toml            # Manifiesto del proyecto Rust
├── Cargo.lock            # Versiones exactas de dependencias
├── src/                  # Código fuente principal
│   ├── main.rs           # Punto de entrada + construcción de la UI GTK4
│   ├── ffmpeg_interface.rs   # Control de procesos ffmpeg (video/audio)
│   ├── wayland_record.rs     # Grabación en Wayland vía PipeWire/GStreamer
│   ├── area_capture.rs       # Captura de área/ventana con xwininfo (Xorg)
│   ├── config_management.rs  # Lectura/escritura de configuración INI
│   ├── timer.rs              # Temporizador de cuenta atrás y cronómetro
│   ├── utils.rs              # Utilidades (detección Wayland/Snap)
│   └── styles/
│       └── global.css        # Estilos CSS para la ventana GTK4
├── interfaces/
│   └── main.ui           # Interfaz gráfica en formato XML (GTK Builder)
├── data/                 # Iconos SVG y PNG de la aplicación
├── po/                   # Archivos de traducción (.po / .mo)
├── flatpak/              # Manifiesto Flatpak
│   ├── sa.sy.bluerecorder.desktop
│   └── sa.sy.bluerecorder.metainfo.xml
├── snap/
│   └── snapcraft.yaml    # Manifiesto Snap
└── packaging/            # Scripts/configs adicionales de empaquetado
```

---

## 2. Requisitos del sistema

### Herramientas de desarrollo

| Herramienta | Versión mínima | Función |
|---|---|---|
| Rust / Cargo | 1.56+ (edition 2021) | Compilador y gestor de paquetes |
| clang | cualquiera | Requerido por `gtk4-rs` (bindgen) |
| pkg-config | cualquiera | Localizar bibliotecas del sistema |
| gettext | cualquiera | Compilar archivos de traducción |

### Bibliotecas del sistema (tiempo de compilación y ejecución)

```bash
# Ubuntu / Debian
sudo apt install \
  build-essential clang pkg-config \
  libgtk-4-dev \
  libgtk-3-dev \
  libgdk-pixbuf-2.0-dev \
  libglib2.0-dev \
  gettext \
  ffmpeg \
  libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev \
  libpipewire-0.3-dev \
  pulseaudio \
  x11-utils          # provee xwininfo (solo Xorg)
```

```bash
# Fedora / RHEL
sudo dnf install \
  clang gtk4-devel glib2-devel gettext \
  ffmpeg gstreamer1-devel \
  gstreamer1-plugins-base-devel \
  gstreamer1-plugins-bad-free-devel \
  pipewire-devel pulseaudio xwininfo
```

---

## 3. Dependencias (crates)

| Crate | Versión | Propósito |
|---|---|---|
| `gtk` (gtk4) | 0.4.6 | Widgets de interfaz gráfica, builder de UI, estilos CSS |
| `gdk` (gdk4) | git | Capa de abstracción de pantalla/entrada de GTK4 |
| `gio` | 0.15 | Sistema de archivos GLib, diálogos nativos |
| `glib` | 0.10 | Tipos y runtime de GLib (`MainContext`, `clone!`, `timeout`) |
| `gtk-sys` | 0.15 | Bindings de bajo nivel de GTK |
| `gdk-pixbuf` | 0.9 | Carga de imágenes (iconos PNG) |
| `gstreamer` | 0.20 | Pipeline multimedia para grabación en Wayland (PipeWire → WebM) |
| `zbus` | 3.12 | Cliente D-Bus asíncrono; comunica con `org.freedesktop.portal.ScreenCast` |
| `async-std` | 1.12 | Runtime asíncrono (punto de entrada `#[async_std::main]`) |
| `chrono` | 0.4 | Generación de nombres de archivo con fecha/hora UTC |
| `dirs` | 4.0 | Rutas estándar del usuario (`home_dir`) |
| `rust-ini` | 0.16 | Lectura y escritura del archivo de configuración INI |
| `gettext-rs` | 0.7 | Internacionalización (bindtextdomain, gettext) |
| `regex` | 1.4 | Parseo de la salida de `xwininfo` para captura de área |
| `subprocess` | 0.2 | Ejecución del comando definido por el usuario tras la grabación |
| `secfmt` | 0.1 | Formateo de segundos en HH:MM:SS para el temporizador |
| `dark-light` | 1.0 | Detección del tema oscuro/claro del sistema |
| `tempfile` | 3.10 | Archivos temporales para video Wayland antes de conversión |
| `filename` | 0.1 | Manipulación de nombres de archivo |

---

## 4. Módulos del código fuente

### `main.rs`
Punto de entrada de la aplicación. Inicializa GTK4, carga la UI desde `interfaces/main.ui` (via `Builder`), conecta las señales de todos los widgets y lanza el `GtkApplication`.

Responsabilidades clave:
- Detectar Wayland vs Xorg (`utils::is_wayland()`) para habilitar/deshabilitar controles.
- Instanciar `Ffmpeg` (struct de control de grabación).
- Conectar los botones Grabar / Detener / Play.
- Cargar y aplicar CSS desde `src/styles/global.css`.
- Mostrar iconos correctos según tema oscuro/claro.

### `ffmpeg_interface.rs`
Contiene la struct `Ffmpeg` y sus métodos:

| Método | Descripción |
|---|---|
| `start_record(x, y, w, h)` | Lanza procesos `ffmpeg` para video (x11grab) y/o audio (pulseaudio). En Wayland delega a `WaylandRecorder`. |
| `stop_record()` | Mata los procesos, mezcla video+audio si ambos están activos, convierte formatos, limpia temporales. |
| `play_record()` | Abre el archivo grabado con `xdg-open` (o `snapctl user-open` dentro de Snap). |

**Flujo de archivos temporales:**
- Video Xorg → `<destino>.temp.without.audio.<ext>`
- Audio → `<destino>.temp.audio`
- Al finalizar: mezcla con `ffmpeg` y elimina los temporales.

### `wayland_record.rs`
Implementa `WaylandRecorder` usando el protocolo **XDG Desktop Portal ScreenCast** vía D-Bus (`zbus`) y el pipeline GStreamer PipeWire.

Flujo:
1. Conecta al bus de sesión D-Bus.
2. Llama a `ScreenCast.CreateSession` → `SelectSources` → `Start`.
3. Obtiene el `node_id` del stream PipeWire.
4. Lanza pipeline GStreamer: `pipewiresrc → videorate → videoconvert → vp8enc → webmmux → filesink`.
5. Al detener: `set_state(Null)` + cierra la sesión D-Bus.

### `area_capture.rs`
Struct `AreaCapture` que usa `xwininfo` (herramienta X11) para obtener coordenadas. Tres modos:
- **Pantalla completa**: `xwininfo -root`  
- **Área seleccionada**: `xwininfo` (el usuario hace clic)  
- **Por nombre de ventana**: `xwininfo -name <nombre>`

Usa `regex` para extraer `Absolute upper-left X/Y`, `Width`, `Height` de la salida de texto.

### `config_management.rs`
Gestiona el archivo `~/.local/share/blue-recorder/config.ini` usando `rust-ini`.

| Función | Descripción |
|---|---|
| `initialize()` | Crea el archivo si no existe con valores por defecto. |
| `get(section, key)` | Lee un valor string del INI. |
| `get_bool(section, key)` | Lee `"1"` como `true`, cualquier otra cosa como `false`. |
| `set(section, key, value)` | Escribe un valor y persiste el archivo. |
| `set_bool(section, key, value)` | Escribe `"1"` o `"0"`. |
| `merge_previous_version()` | Migra configuración antigua (reemplaza `Options`→`default`, `True`/`False`→`1`/`0`). |

### `timer.rs`
- `recording_delay()`: muestra una ventana de cuenta atrás usando `glib::timeout_add_seconds_local`.
- `start_timer()`: actualiza el label en formato `HH:MM:SS` cada segundo mientras la label es visible.
- `stop_timer()`: reinicia la label a `00:00:00`.

### `utils.rs`
- `is_wayland()`: comprueba si `$WAYLAND_DISPLAY` está definida.
- `is_snap()`: comprueba si `$SNAP` está definida.

---

## 5. Cómo compilar

### Compilación de desarrollo (con símbolos de debug)
```bash
cd /home/jean/Programacion/Rust/blue-recorder
cargo build
```
El binario queda en `target/debug/blue-recorder`.

### Compilación de producción (optimizada)
```bash
cargo build --release
```
El binario queda en `target/release/blue-recorder`.

### Preparar datos junto al binario
El programa busca los directorios `data/`, `interfaces/` y `po/` **junto al binario**:
```bash
cargo build --release
cp -a data interfaces po target/release/
```

---

## 6. Cómo ejecutar en desarrollo

Desde el directorio raíz del proyecto (necesario para que encuentre `data/` y `po/` con las variables de entorno):

```bash
# Opción 1: cargo run (busca data/ por variable de entorno)
DATA_DIR=data/ PO_DIR=po cargo run

# Opción 2: compilar y ejecutar con cp
cargo build
cp -a data interfaces po target/debug/
./target/debug/blue-recorder
```

Para ver logs de GStreamer:
```bash
GST_DEBUG=3 DATA_DIR=data/ PO_DIR=po cargo run
```

---

## 7. Variables de entorno útiles

| Variable | Descripción |
|---|---|
| `DATA_DIR` | Ruta al directorio `data/` con iconos (por defecto: `data/`) |
| `PO_DIR` | Ruta al directorio de traducciones (por defecto: `po`) |
| `DISPLAY` | Display Xorg (por defecto: `:0`) |
| `WAYLAND_DISPLAY` | Si está definida, activa el modo Wayland |
| `SNAP` | Si está definida, activa el modo Snap (usa `snapctl user-open`) |
| `GST_DEBUG` | Nivel de debug de GStreamer (0-9) |

---

## 8. Flujo de grabación

```
Usuario presiona [Record]
        │
        ├─ ¿Delay > 0? → Mostrar cuenta atrás → al terminar, continuar
        │
        ├─ Xorg + Video activo?
        │     └─ Lanzar ffmpeg -f x11grab ... → video_process (Child)
        │
        ├─ Wayland + Video activo?
        │     └─ WaylandRecorder.start() → D-Bus ScreenCast Portal
        │           └─ GStreamer pipeline (pipewiresrc → vp8enc → webm)
        │
        └─ Audio activo?
              └─ Lanzar ffmpeg -f pulse ... → audio_process (Child)

Usuario presiona [Stop]
        │
        ├─ kill video_process (Xorg) | pipeline.set_state(Null) (Wayland)
        ├─ kill audio_process
        │
        ├─ ¿Video + Audio?
        │     └─ ffmpeg -i video.temp -i audio.temp -c:a aac output.ext
        │
        ├─ ¿Solo Audio?
        │     └─ ffmpeg -f ogg -i audio.temp output.ext
        │
        ├─ ¿Solo Video Wayland?
        │     └─ ffmpeg -i temp.webm output.ext (conversión de formato)
        │
        ├─ Eliminar archivos temporales
        └─ Ejecutar comando post-grabación (si está definido)
```

---

## 9. Configuración persistente

El archivo de configuración se guarda en:
```
~/.local/share/blue-recorder/config.ini
```

Ejemplo de contenido:
```ini
[default]
frame = 60
delay = 0
folder = file:///home/usuario/Videos
command =
filename =
videocheck = 1
audiocheck = 1
mousecheck = 1
followmousecheck = 0
hidecheck = 0
```

Cada vez que el usuario cambia un control, el valor se persiste inmediatamente mediante `config_management::set()`.

---

## 10. Internacionalización (i18n)

Los archivos de traducción están en `po/`. El sistema usa `gettext-rs`:

```bash
# Extraer cadenas traducibles (requiere xgettext)
xgettext --from-code=UTF-8 -k'gettext' -o po/blue-recorder.pot src/*.rs

# Compilar traducciones
msgfmt po/es.po -o po/es/LC_MESSAGES/blue-recorder.mo
```

Para añadir un nuevo idioma:
1. Copiar `po/blue-recorder.pot` a `po/<codigo>.po`
2. Traducir los `msgstr` vacíos
3. Compilar con `msgfmt`

---

## 11. Empaquetado

### Script Unificado (Recomendado)

Se ha creado el script `build.sh` en la raíz del proyecto para simplificar y automatizar la compilación, generación de archivos `.mo` de traducción, y empaquetado del software en sus formatos de distribución habituales.

```bash
# Dar permisos de ejecución la primera vez
chmod +x build.sh

# Compilar todo: Binario de Release + Traducciones + .deb + AppImage + Flatpak
./build.sh

# Se pueden pasar flags específicas para construir solo los formatos deseados:
./build.sh --deb
./build.sh --appimage
./build.sh --flatpak
```

Todos los artefactos generados (incluyendo las conversiones locales de la internacionalización y los paquetes finales) se guardarán organizadamente dentro del nuevo directorio `build_output/`.

#### Construcción Universal con Docker (Recomendado para Releases)

Construir en una máquina con una versión muy nueva de Linux puede hacer que el binario generado (y los AppImage) fallen en sistemas operativos ligeramente más antiguos dando errores como `version 'GLIBC_2.39' not found`. Para distribuir el software masivamente es recomendable compilar sobre un sistema con una librería libc antigua, como Ubuntu 22.04 LTS.

Para ello, usa el script encapsulado mediante Docker:
```bash
chmod +x build-docker.sh

# Se comporta igual que build.sh, pero todo corre aislado en Ubuntu 22.04
./build-docker.sh --appimage --deb
```

### Snap

El manifiesto está en `snap/snapcraft.yaml`. Usa la extensión `gnome` de Snapcraft y `confinement: strict`.

```bash
# Instalar snapcraft
sudo snap install snapcraft --classic

# Construir el paquete (dentro del directorio del proyecto)
snapcraft

# El resultado es un archivo .snap en el directorio actual
# Instalar localmente para probar:
sudo snap install blue-recorder_*.snap --dangerous
```

**Plugs configurados en el snap:** `desktop`, `home`, `audio-playback`, `audio-record`, `wayland`, `x11`, `pipewire`, `screencast-legacy`.

El snap incluye PipeWire compilado desde fuente (versión 0.3.32 via Meson).

### Flatpak (Manual)

El script `build.sh` con la opción `--flatpak` invoca `flatpak-builder` automáticamente generando un manifiesto temporal subyacente. Sin embargo, si deseas construir Flatpak manualmente con archivos pre-existentes:

Los archivos de metadatos están en `flatpak/`:
- `sa.sy.bluerecorder.desktop` — entrada de escritorio
- `sa.sy.bluerecorder.metainfo.xml` — metadatos para Flathub

```bash
# Instalar flatpak-builder
sudo apt install flatpak-builder

# Construir (se necesita un manifiesto JSON/YAML en la raíz)
flatpak-builder --force-clean build-dir sa.sy.bluerecorder.json

# Instalar localmente
flatpak-builder --user --install --force-clean build-dir sa.sy.bluerecorder.json

# Ejecutar
flatpak run sa.sy.bluerecorder
```

---

## 12. Distribución

| Canal | Estado | Pasos |
|---|---|---|
| **Snap Store** | Publicado (`blue-recorder`) | `snapcraft upload blue-recorder_*.snap --release=stable` |
| **Flathub** | Publicado (`sa.sy.bluerecorder`) | PR al repositorio [flathub/sa.sy.bluerecorder](https://github.com/flathub/sa.sy.bluerecorder) |
| **Binario manual** | Posible | `cargo build --release` + copiar `data/`, `interfaces/`, `po/` junto al binario |
| **AUR (Arch Linux)** | Potencial | Crear `PKGBUILD` apuntando al release de GitHub |

Para crear un release en GitHub:
1. Actualizar la versión en `Cargo.toml`  
2. Hacer commit y crear tag: `git tag v0.3.0 && git push --tags`
3. La CI/CD (si existe en `.github/`) puede automatizar la construcción del snap/flatpak.

---

## 13. Cómo mejorar el proyecto

### Mejoras técnicas prioritarias

1. **Migrar a GTK4 nativo completo**  
   Algunos widgets todavía usan patrones de GTK3. Revisar el uso de `MessageDialog` (deprecado en GTK4.10) y reemplazar con `AlertDialog`.

2. **Reemplazar `kill` de sistema por señales propias**  
   En `ffmpeg_interface.rs`, el proceso se mata con `Command::new("kill")`. Usar el método nativo de Rust `child.kill()` es más portable y seguro.

3. **Evitar bloqueo del hilo principal**  
   `self.main_context.block_on(...)` en el hilo de UI puede congelar la interfaz. Migrar a `spawn_local` con callbacks GTK o canales `glib::MainContext::channel`.

4. **Agregar pruebas unitarias**  
   Actualmente no hay tests. Candidatos iniciales:
   - `config_management`: escribir, leer y migrar configuración.
   - `timer`: formateo de segundos.
   - `area_capture`: parseo de regex con salidas de ejemplo de `xwininfo`.

5. **Soporte para múltiples monitores**  
   En Wayland, el portal ScreenCast permite seleccionar monitor; en Xorg se puede especificar el output con `xrandr`. Exponer esto en la UI.

6. **Soporte para audio desde Pipewire**  
   Actualmente el audio usa PulseAudio (`-f pulse`). Para distros modernas (Fedora, Arch) conviene añadir soporte directo con PipeWire (`-f pipewire`).

7. **Añadir CI/CD**  
   Crear `.github/workflows/build.yml` que compile y ejecute tests en cada push.

8. **Mejorar manejo de errores**  
   Reemplazar los `unwrap()` generalizados con manejo explícito de errores (`?`, `match`, `if let`) para mayor robustez.

9. **Reducir clones innecesarios**  
   La struct `Ffmpeg` se clona repetidamente (`.clone()`) porque GTK requiere `'static` closures. Considerar `Arc<Mutex<Ffmpeg>>` o reestructurar el estado.

### Ideas de nuevas funcionalidades

- 🎚️ Control de calidad/bitrate de video desde la UI
- 📸 Captura de pantalla (screenshot) además de video
- ⏸️ Pausa y reanudación de la grabación
- 🎬 Previsualización en tiempo real de la región seleccionada
- 🔔 Notificación de sistema al terminar la grabación
- 📂 Historial de grabaciones recientes

---

## 14. Convenciones de código

- **Edición Rust 2021** — usar `edition = "2021"` en Cargo.toml.
- **Nombrado**: `snake_case` para variables y funciones, `PascalCase` para structs y enums.
- **Módulos**: cada archivo `.rs` en `src/` es un módulo declarado en `main.rs` con `mod`.
- **Formateo**: usar `cargo fmt` antes de cada commit.
- **Linting**: ejecutar `cargo clippy` y corregir los `warn` antes de abrir un PR.
- **UI**: editar `interfaces/main.ui` con Glade o GNOME Builder (es XML de GTK Builder).
- **CSS**: los estilos globales están en `src/styles/global.css`, cargados en tiempo de compilación con `include_str!`.

```bash
# Comandos de mantenimiento habituales
cargo fmt          # formatear código
cargo clippy       # detectar problemas de estilo/bugs
cargo test         # ejecutar tests
cargo audit        # revisar vulnerabilidades en dependencias (requiere: cargo install cargo-audit)
cargo outdated     # verificar dependencias desactualizadas (requiere: cargo install cargo-outdated)
```
