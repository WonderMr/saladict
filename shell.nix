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

    # Build tools
    dpkg
  ];

  shellHook = ''
    export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
  '';
}
