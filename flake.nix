{
  description = "Form Generator - Generate a small form website to collect answers, from a TOML config file. ";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.stable.latest.default;
        openssl = pkgs.openssl;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "form-generator";
          version = "0.1.1";

          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            rust
            pkgs.pkg-config
          ];

          buildInputs = [
            openssl
          ];

          OPENSSL_LIB_DIR = "${openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${openssl.dev}/include";
          PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rust
            pkgs.cargo
            pkgs.rust-analyzer
            pkgs.pkg-config
            openssl
          ];
          OPENSSL_DIR = openssl.dev;
          PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";
        };
      });
}

