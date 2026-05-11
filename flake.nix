{
  description = "Nix flake for CAPS";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {self, ...} @ inputs: let
    supportedSystems = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
    forEachSupportedSystem = f:
      inputs.nixpkgs.lib.genAttrs supportedSystems (
        system:
          f {
            inherit system;
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.self.overlays.default
              ];
            };
          }
      );

    runtimeDeps = pkgs:
      with pkgs; [
        fontconfig
      ];
  in {
    overlays.default = final: prev: {
      rustToolchain = with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
        combine (
          with stable; [
            clippy
            rustc
            cargo
            rustfmt
            rust-src
          ]
        );
    };

    devShells = forEachSupportedSystem (
      {
        pkgs,
        system,
      }: {
        default = pkgs.mkShell {
          packages = with pkgs;
            [
              rustToolchain
              openssl
              pkg-config
              cargo-deny
              cargo-edit
              cargo-watch
              rust-analyzer
              self.formatter.${system}
            ]
            ++ (runtimeDeps pkgs);

          env = {
            # Required by rust-analyzer
            RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      }
    );

    formatter = forEachSupportedSystem ({pkgs, ...}: pkgs.nixfmt);

    packages = forEachSupportedSystem (
      {
        pkgs,
        system,
      }: let
        lib = pkgs.lib;
      in {
        caps = pkgs.rustPlatform.buildRustPackage {
          pname = "caps";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = runtimeDeps pkgs;

          meta = {
            description = "A simple, cross-platform graphical tool that rewards consistent typing on with points";
            homepage = "https://github.com/megabyte6/caps";
            license = lib.licenses.gpl3;
          };
        };

        default = self.packages.${system}.caps;
      }
    );
  };
}
