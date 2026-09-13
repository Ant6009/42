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
      pkg = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "42";
        version = "0.1.0";
        src = self;
        cargoBuildOptions = "";
        # rusqlite is bundled; no system sqlite needed at build time.
        nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.pkg-config ];
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
        options.services.fortytwo = {
          enable = { type = "bool"; default = false; description = "Enable the 42 service."; };
          configPath = {
            type = "path";
            default = "/etc/42/42.toml";
            description = "Path to the 42 TOML config file.";
          };
        };

        config = lib.mkIf config.services.fortytwo.enable {
          systemd.services."42" = {
            wantedBy = [ "multi-user.target" ];
            after = [ "network.target" ];
            serviceConfig = {
              ExecStart = "${pkgs.callPackage pkg {}}/bin/42 --config ${config.services.fortytwo.configPath}";
              Restart = "on-failure";
            };
            user = "42";
            group = "42";
          };

          users.users."42" = {
            isSystemUser = true;
            group = "42";
            description = "42 service user";
          };
          users.groups."42" = { };

          # NOTE: the SQLite path and /etc/42/42.toml must be writable by the
          # service user; adjust StateDirectory or permissions in the config.
          environment.etc."42/42.toml".source = config.services.fortytwo.configPath;
        };
      };
    };
}
