{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flakeUtils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk/master";
  };

  outputs = {
    nixpkgs,
    flakeUtils,
    naersk,
    ...
  }:
    flakeUtils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
      naerskLib = pkgs.callPackage naersk {};
    in {
      defaultPackage = naerskLib.buildPackage ./.;
      devShell = with pkgs;
        mkShell {
          buildInputs = [
            cargo
            rustc
            rustfmt
            rustPackages.clippy
          ];

          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
    });
}
