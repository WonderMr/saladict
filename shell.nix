{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    gobject-introspection
    cargo
    rustc
    nodejs
    pnpm
    gcc
  ];

  buildInputs = with pkgs; [
    # Tauri v2 core
    webkitgtk_4_1
    libsoup_3
    gtk3
    glib
    cairo
    pango
    gdk-pixbuf
    atk
    harfbuzz

    # System deps
    openssl
    dbus
    librsvg

    # Project-specific
    xdotool
    xorg.libxcb
    xorg.libXrandr
    xorg.libX11
    xorg.libXi
    xorg.libXtst
    tesseract

    # Tray icon
    libayatana-appindicator

    # Build tools
    dpkg
    patchelf
  ];

  shellHook = ''
    export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
    # Workarounds for webkit2gtk-4.1 issues on NixOS
    export GDK_BACKEND=x11              # Wayland protocol error 71 on multi-window
    export WEBKIT_DISABLE_DMABUF_RENDERER=1  # GBM buffer allocation failure
    export WEBKIT_DISABLE_COMPOSITING_MODE=1 # Additional rendering stability
    # Append to LD_LIBRARY_PATH only if it's already set so we don't leave an
    # empty trailing entry (which the dynamic linker would treat as CWD).
    export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
      pkgs.webkitgtk_4_1
      pkgs.libsoup_3
      pkgs.gtk3
      pkgs.glib
      pkgs.openssl
      pkgs.dbus
      pkgs.libayatana-appindicator
      pkgs.xdotool
    ]}''${LD_LIBRARY_PATH:+:''$LD_LIBRARY_PATH}"
  '';
}
