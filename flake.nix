{
  inputs = {
    nixpkgs = {
      type = "indirect";
      id = "nixpkgs";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    naersk = {
      url = "github:nix-community/naersk/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-parts,
    naersk,
    fenix,
  }:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];

      perSystem = {
        inputs',
        config,
        system,
        pkgs,
        ...
      }: let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        toolchain = fenix.packages."${system}".fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-4ublOInImJyXpU1kMUSsAOUxT0fCuYgcs7M1i6N4L3k=";
        };

        naersk' = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };
      in {
        packages.default = naersk'.buildPackage ./.;

        formatter = pkgs.alejandra;

        devShells.default = with pkgs;
          mkShell {
            buildInputs = [
              toolchain
              cargo-expand

              fuse3
              fuse3.dev
              pkg-config
              sqlite
              openssl
              openssl.dev
              s2n-tls
            ];

            RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
            LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.fuse3 openssl s2n-tls ];
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.lib.getLib pkgs.openssl}/lib";
          };
      };
    };
}
