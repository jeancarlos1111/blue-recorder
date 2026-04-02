#!/bin/bash
set -e

# blue-recorder build automation script

# Variables
APP_NAME="blue-recorder"
BIN_NAME="blue-recorder"
VERSION=$(grep -m1 '^version =' Cargo.toml | awk -F '"' '{print $2}')
ARCH=$(uname -m)
BUILD_DIR="build_output"

if [ -z "$VERSION" ]; then
    echo "Error: Could not determine version from Cargo.toml"
    exit 1
fi

echo "========================================"
echo "Building $APP_NAME v$VERSION ($ARCH)"
echo "========================================"

# Clean previous build dir
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# 1. Compile Rust Release
echo "--> Compiling Rust project (Release)..."
cargo build --release
cp target/release/"$BIN_NAME" "$BUILD_DIR/"

# 2. Compile Translations (.po to .mo)
echo "--> Compiling translations..."
mkdir -p "$BUILD_DIR/locale"
for po_file in po/*.po; do
    lang=$(basename "$po_file" .po)
    msgdir="$BUILD_DIR/locale/$lang/LC_MESSAGES"
    mkdir -p "$msgdir"
    msgfmt "$po_file" -o "$msgdir/$APP_NAME.mo"
done

# Check arguments to determine what to package
BUILD_DEB=false
BUILD_APPIMAGE=false
BUILD_FLATPAK=false

if [ "$#" -eq 0 ]; then
    echo "No target specified. Building all targets."
    BUILD_DEB=true
    BUILD_APPIMAGE=true
    BUILD_FLATPAK=true
else
    for arg in "$@"; do
        case $arg in
            --deb) BUILD_DEB=true ;;
            --appimage) BUILD_APPIMAGE=true ;;
            --flatpak) BUILD_FLATPAK=true ;;
            *) echo "Unknown target: $arg"; exit 1 ;;
        esac
    done
fi

# 3. Build Debian Package
if [ "$BUILD_DEB" = true ]; then
    echo "--> Building Debian package (.deb)..."
    DEB_DIR="$BUILD_DIR/deb_package"
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/usr/share/applications"
    mkdir -p "$DEB_DIR/usr/share/pixmaps"
    mkdir -p "$DEB_DIR/usr/share/locale"
    mkdir -p "$DEB_DIR/opt/$APP_NAME/interfaces"
    mkdir -p "$DEB_DIR/opt/$APP_NAME/data"
    
    # Copy assets
    cp "$BUILD_DIR/$BIN_NAME" "$DEB_DIR/usr/bin/"
    cp "data/blue-recorder.desktop" "$DEB_DIR/usr/share/applications/"
    cp "data/blue-recorder.svg" "$DEB_DIR/usr/share/pixmaps/"
    cp -r "interfaces/"* "$DEB_DIR/opt/$APP_NAME/interfaces/"
    cp -r "data/"* "$DEB_DIR/opt/$APP_NAME/data/"
    cp -r "$BUILD_DIR/locale/"* "$DEB_DIR/usr/share/locale/"
    
    # Create DEBIAN/control file
    # We map x86_64 to amd64 for Debian
    DEB_ARCH=$ARCH
    if [ "$DEB_ARCH" = "x86_64" ]; then DEB_ARCH="amd64"; fi
    
    cat > "$DEB_DIR/DEBIAN/control" <<EOF
Package: $APP_NAME
Version: $VERSION
Section: video
Priority: optional
Architecture: $DEB_ARCH
Depends: libc6, libglib2.0-0, libgtk-3-0, ffmpeg, pulseaudio | pipewire-pulse, x11-utils
Maintainer: Salem Yaslem <xlmnxp@protonmail.com>
Description: A simple desktop recorder for Linux systems.
 Built using Rust, GTK+ 3 and ffmpeg. It supports recording audio and
 video on almost all Linux interfaces with support for Wayland display server.
EOF

    # Build the package
    dpkg-deb --build "$DEB_DIR" "$BUILD_DIR/${APP_NAME}_${VERSION}_${DEB_ARCH}.deb"
    echo "Created: $BUILD_DIR/${APP_NAME}_${VERSION}_${DEB_ARCH}.deb"
fi

# 4. Build AppImage
if [ "$BUILD_APPIMAGE" = true ]; then
    echo "--> Building AppImage..."
    APPDIR="$BUILD_DIR/AppDir"
    mkdir -p "$APPDIR/usr/bin"
    mkdir -p "$APPDIR/usr/share/applications"
    mkdir -p "$APPDIR/usr/share/pixmaps"
    mkdir -p "$APPDIR/usr/share/locale"
    
    # Copy binary and local assets for AppDir
    cp "$BUILD_DIR/$BIN_NAME" "$APPDIR/usr/bin/"
    cp -r "data" "$APPDIR/usr/bin/"
    cp -r "interfaces" "$APPDIR/usr/bin/"
    cp -r "$BUILD_DIR/locale/"* "$APPDIR/usr/share/locale/"
    
    cp "data/blue-recorder.desktop" "$APPDIR/usr/share/applications/"
    cp "data/blue-recorder.svg" "$APPDIR/"
    cp "data/blue-recorder.svg" "$APPDIR/usr/share/pixmaps/"
    
    # Ensure correct Desktop file configuration for AppImage
    cat > "$APPDIR/blue-recorder.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Blue Recorder
Icon=blue-recorder
Exec=blue-recorder
Categories=AudioVideo;GTK;
Comment=A simple desktop recorder for Linux systems. Built using GTK+ 3 and ffmpeg.
EOF

    # Download appimagetool if not exists
    if [ ! -f "appimagetool-x86_64.AppImage" ]; then
        echo "Downloading appimagetool..."
        wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
        chmod +x appimagetool-x86_64.AppImage
    fi

    # Create AppRun
    cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export DATA_DIR="${HERE}/usr/bin/data"
export INTERFACES_DIR="${HERE}/usr/bin/interfaces"
export TEXTDOMAINDIR="${HERE}/usr/share/locale"
exec "${HERE}/usr/bin/blue-recorder" "$@"
EOF
    chmod +x "$APPDIR/AppRun"
    
    # Build AppImage
    APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 ./appimagetool-x86_64.AppImage "$APPDIR" "$BUILD_DIR/${APP_NAME}-${VERSION}-x86_64.AppImage"
    echo "Created: $BUILD_DIR/${APP_NAME}-${VERSION}-x86_64.AppImage"
fi

# 5. Build Flatpak
if [ "$BUILD_FLATPAK" = true ]; then
    echo "--> Building Flatpak..."
    # The application does not currently have a flatpak json inside flatpak/, 
    # but the instructions requested flatpak building. We will generate a basic manifest.
    FLATPAK_DIR="$BUILD_DIR/flatpak_build"
    mkdir -p "$FLATPAK_DIR"
    
    MANIFEST="$FLATPAK_DIR/sa.sy.bluerecorder.json"
    cat > "$MANIFEST" <<EOF
{
    "app-id": "sa.sy.bluerecorder",
    "runtime": "org.gnome.Platform",
    "runtime-version": "44",
    "sdk": "org.gnome.Sdk",
    "sdk-extensions": [
        "org.freedesktop.Sdk.Extension.rust-stable"
    ],
    "command": "blue-recorder",
    "finish-args": [
        "--share=ipc",
        "--socket=fallback-x11",
        "--socket=wayland",
        "--socket=pulseaudio",
        "--talk-name=org.freedesktop.portal.Desktop",
        "--filesystem=home"
    ],
    "build-options": {
        "append-path": "/usr/lib/sdk/rust-stable/bin",
        "env": {
            "CARGO_HOME": "/run/build/blue-recorder/cargo"
        }
    },
    "modules": [
        {
            "name": "blue-recorder",
            "buildsystem": "simple",
            "build-commands": [
                "cargo build --release",
                "install -Dm755 target/release/blue-recorder /app/bin/blue-recorder",
                "install -d /app/share/blue-recorder",
                "cp -r data /app/share/blue-recorder/",
                "cp -r interfaces /app/share/blue-recorder/",
                "install -Dm644 data/blue-recorder.desktop /app/share/applications/sa.sy.bluerecorder.desktop",
                "install -Dm644 data/blue-recorder.svg /app/share/icons/hicolor/scalable/apps/sa.sy.bluerecorder.svg",
                "install -Dm644 flatpak/sa.sy.bluerecorder.metainfo.xml /app/share/metainfo/sa.sy.bluerecorder.metainfo.xml"
            ],
            "sources": [
                {
                    "type": "dir",
                    "path": "$(pwd)"
                }
            ]
        }
    ]
}
EOF

    echo "Building Flatpak (this might take a while to download runtimes if not cached)..."
    # We use flatpak-builder
    if command -v flatpak-builder &> /dev/null; then
         flatpak-builder --force-clean "$FLATPAK_DIR/build" "$MANIFEST"
         flatpak build-export "$FLATPAK_DIR/repo" "$FLATPAK_DIR/build"
         flatpak build-bundle "$FLATPAK_DIR/repo" "$BUILD_DIR/${APP_NAME}-${VERSION}.flatpak" sa.sy.bluerecorder
         echo "Created: $BUILD_DIR/${APP_NAME}-${VERSION}.flatpak"
    else
         echo "Warning: flatpak-builder is not installed. Skipping flatpak build."
    fi
fi

echo "========================================"
echo "Build complete! Check the $BUILD_DIR directory."
echo "========================================"
