{ pkgs, ... }:

{
  packages = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rust-analyzer
    pkg-config
    # Build deps for crates that compile C (rusqlite bundled, ring, etc.)
    cc
    sqlite
    # Playwright e2e
    nodejs
    # Dev utilities
    just
    curl
  ];

  languages.rust = {
    enable = true;
    rustfmt = true;
    clippy = true;
  };

  enterShell = ''
    alias l=just
  '';
}
