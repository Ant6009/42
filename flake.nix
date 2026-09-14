{
  description = "42 — standalone local-network Perplexity clone";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      lib = nixpkgs.lib;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in f pkgs
      );
      pkg = pkgs:
      let
        rustPlatform = pkgs.makeRustPlatform {
          rustc = pkgs.rustc;
          cargo = pkgs.cargo;
        };
      in
      rustPlatform.buildRustPackage {
        pname = "42";
        version = "0.1.0";
        src = self;
        cargoLock = { lockFile = ./Cargo.lock; };   # committed lockfile
        cargoBuildOptions = "";
        # rusqlite is bundled, TLS is rustls; no system deps needed at
        # build time (cc comes from stdenv).
        nativeBuildInputs = [ pkgs.pkg-config ];
        meta.description = "Standalone local-network Perplexity clone";
      };
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkg pkgs;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.rust-analyzer
            pkgs.pkg-config
            pkgs.cc
            pkgs.sqlite
            pkgs.nodejs
            pkgs.just
          ];
        };
      });

      nixosModules.default = { config, pkgs, ... }:
      {
        options.services.fortytwo.enable = lib.mkOption {
          default = false;
          type = lib.types.bool;
          description = "Enable the 42 service.";
        };
        options.services.fortytwo.configPath = lib.mkOption {
          type = lib.types.path;
          default = "/etc/42/42.toml";
          description = "Path to the 42 TOML config file.";
        };

        config = lib.mkIf config.services.fortytwo.enable {
          systemd.services."42" = {
            wantedBy = [ "multi-user.target" ];
            after = [ "network-online.target" ];
            wants = [ "network-online.target" ];
            serviceConfig = {
              ExecStart = "${pkg pkgs}/bin/42 --config ${config.services.fortytwo.configPath}";
              Restart = "on-failure";
              RestartSec = 3;
              StateDirectory = "42";   # /var/lib/42 for the SQLite DB
              StateDirectoryMode = "0750";
              User = "fortytwo";       # numeric usernames are rejected by shadow
              Group = "fortytwo";
            };
          };

          users.users.fortytwo = {
            isSystemUser = true;
            group = "fortytwo";
            home = "/var/lib/42";
            description = "42 service user";
          };
          users.groups.fortytwo = { };

          # The config file itself is managed outside this module (e.g. by the
          # host's own environment.etc or agenix); it must be readable by the
          # service user.
        };
      };
    };
}
