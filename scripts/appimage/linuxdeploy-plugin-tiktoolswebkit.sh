#!/usr/bin/env bash
# linuxdeploy input plugin: make the bundled WebKitGTK find its helper processes.
#
# libwebkit2gtk-4.1 spawns WebKitNetworkProcess/WebKitWebProcess and loads its
# injected bundle from an absolute directory fixed at build time (Debian/Ubuntu:
# /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1, Arch: /usr/lib/webkit2gtk-4.1).
# Release builds ignore WEBKIT_EXEC_PATH, so an AppImage built on Ubuntu aborts
# with SIGTRAP on any distribution that keeps the helpers elsewhere.
#
# This plugin copies the helpers into the AppDir and rewrites the bundled
# library's path from "/usr/lib/..." to the same-length "././/lib/...". The AppImageKit
# AppRun used by cargo-packager chdirs into "$APPDIR/usr" before exec, so the
# relative path resolves to the bundled copies.
#
# It also drops linuxdeploy's bundled Wayland client libraries. They are pinned
# to the build host's Wayland version and shadow the host's newer stack, which
# makes Mesa's EGL fail inside WebKitWebProcess on rolling distributions
# ("Could not create default EGL display: EGL_BAD_PARAMETER"). Like Mesa, the
# Wayland client stack must always come from the host.
set -euo pipefail

APPDIR=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --plugin-api-version) echo "0"; exit 0 ;;
    --plugin-type) echo "input"; exit 0 ;;
    --appdir) APPDIR="${2:-}"; shift 2 ;;
    *) echo "linuxdeploy-plugin-tiktoolswebkit: invalid argument: $1" >&2; exit 1 ;;
  esac
done
if [ -z "$APPDIR" ]; then
  echo "usage: $0 --appdir <AppDir>" >&2
  exit 1
fi
APPDIR="$(realpath "$APPDIR")"
export LC_ALL=C

libdir="$(pkg-config --variable=libdir webkit2gtk-4.1)"
helper_dir="$libdir/webkit2gtk-4.1"
case "$helper_dir" in
  /usr/*) ;;
  *) echo "linuxdeploy-plugin-tiktoolswebkit: helper dir must live under /usr: $helper_dir" >&2; exit 1 ;;
esac
relative_dir="${helper_dir#/usr/}"
# Same length as the original: "/usr" -> "././" (yields "././/lib/...").
patched_dir="././${helper_dir#/usr}"

for helper in WebKitNetworkProcess WebKitWebProcess injected-bundle/libwebkit2gtkinjectedbundle.so; do
  if [ ! -f "$helper_dir/$helper" ]; then
    echo "linuxdeploy-plugin-tiktoolswebkit: missing host helper $helper_dir/$helper" >&2
    exit 1
  fi
done

target="$APPDIR/usr/$relative_dir"
mkdir -p "$target/injected-bundle"
install -m 0755 "$helper_dir/WebKitNetworkProcess" "$helper_dir/WebKitWebProcess" "$target/"
if [ -f "$helper_dir/WebKitGPUProcess" ]; then
  install -m 0755 "$helper_dir/WebKitGPUProcess" "$target/"
fi
install -m 0755 "$helper_dir/injected-bundle/libwebkit2gtkinjectedbundle.so" "$target/injected-bundle/"

shopt -s nullglob
libraries=("$APPDIR"/usr/lib/libwebkit2gtk-4.1.so*)
shopt -u nullglob
patched=0
for library in "${libraries[@]}"; do
  [ -f "$library" ] && [ ! -L "$library" ] || continue
  if grep -qaF "$helper_dir" "$library"; then
    # Literal, same-length in-place rewrite; the size check guards the ELF layout.
    size="$(stat -c %s "$library")"
    OLD="$helper_dir" NEW="$patched_dir" perl -0777 -pi -e 's/\Q$ENV{OLD}\E/$ENV{NEW}/g' "$library"
    if [ "$(stat -c %s "$library")" != "$size" ]; then
      echo "linuxdeploy-plugin-tiktoolswebkit: patch changed the size of $library" >&2
      exit 1
    fi
    if grep -qaF "$helper_dir" "$library" || ! grep -qaF "$patched_dir" "$library"; then
      echo "linuxdeploy-plugin-tiktoolswebkit: failed to patch $library" >&2
      exit 1
    fi
    patched=$((patched + 1))
  fi
done
if [ "$patched" -eq 0 ]; then
  echo "linuxdeploy-plugin-tiktoolswebkit: no bundled libwebkit2gtk-4.1 referencing $helper_dir" >&2
  exit 1
fi

# GStreamer discovers plugins dynamically: following WebKit's ELF dependencies
# alone only deploys libgstapp-1.0, not the appsrc/appsink factories in libgstapp.
# Use plugins and the scanner from the same build host as the bundled core.
gst_plugins="$(pkg-config --variable=pluginsdir gstreamer-1.0)"
gst_scanners="$(pkg-config --variable=pluginscannerdir gstreamer-1.0)"
gst_target="$APPDIR/usr/lib/gstreamer-1.0"
scanner_target="$APPDIR/usr/libexec/gstreamer-1.0/gst-plugin-scanner"
if [ ! -f "$gst_scanners/gst-plugin-scanner" ]; then
  echo "linuxdeploy-plugin-tiktoolswebkit: missing GStreamer scanner in $gst_scanners" >&2
  exit 1
fi
mkdir -p "$gst_target" "$(dirname "$scanner_target")"
install -m 0755 "$gst_scanners/gst-plugin-scanner" "$scanner_target"
manifest="$(dirname "${BASH_SOURCE[0]}")/gstreamer-plugins.txt"
while IFS= read -r plugin; do
  [ -n "$plugin" ] || continue
  if [ ! -f "$gst_plugins/libgst$plugin.so" ]; then
    echo "linuxdeploy-plugin-tiktoolswebkit: missing libgst$plugin.so; install GStreamer base, good and libav plugins" >&2
    exit 1
  fi
  install -m 0755 "$gst_plugins/libgst$plugin.so" "$gst_target/"
done < "$manifest"
# Deploy codec dependencies without flattening the plugin directory. This child
# has no --plugin option, so it cannot invoke this input plugin recursively.
tiktools_linuxdeploy="$(dirname "${BASH_SOURCE[0]}")/linuxdeploy-x86_64.AppImage"
if [ ! -x "$tiktools_linuxdeploy" ]; then
  echo "linuxdeploy-plugin-tiktoolswebkit: missing dependency deployer $tiktools_linuxdeploy" >&2
  exit 1
fi
"$tiktools_linuxdeploy" --appdir "$APPDIR" --deploy-deps-only "$gst_target" --deploy-deps-only "$scanner_target"

# Keep the host's Wayland client stack (see the header). Mixed versions make
# Mesa's EGL initialization fail in the WebKit web process on newer distros.
shopt -s nullglob
wayland_libraries=(
  "$APPDIR"/usr/lib/libwayland-client.so.0*
  "$APPDIR"/usr/lib/libwayland-cursor.so.0*
  "$APPDIR"/usr/lib/libwayland-egl.so.1*
  "$APPDIR"/usr/lib/libwayland-server.so.0*
)
shopt -u nullglob
for library in "${wayland_libraries[@]}"; do
  rm -f "$library"
done

echo "linuxdeploy-plugin-tiktoolswebkit: bundled helpers into usr/$relative_dir, patched $patched librar(y/ies), removed ${#wayland_libraries[@]} bundled Wayland librar(y/ies)"
