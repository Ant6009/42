{ pkgs, ... }:

{
  packages = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rust-analyzer
    pkg-config
    # Build deps for crates that compile C (rusqlite bundled, ring, etc.)
    gcc
    sqlite
    # Playwright e2e
    nodejs
    # Dev utilities
    just
    curl
  ];

  languages.rust.enable = true;

  enterShell = ''
    alias l=just
  '';
}
