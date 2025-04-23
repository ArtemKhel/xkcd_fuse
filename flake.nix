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

        env = with pkgs; {
          LD_LIBRARY_PATH = lib.makeLibraryPath [fuse3 openssl s2n-tls];
          OPENSSL_DIR = "${openssl.dev}";
          OPENSSL_LIB_DIR = "${lib.getLib openssl}/lib";
          OPENSSL_INCLUDE_DIR = "${openssl.dev}/include";
        };

        buildDeps = with pkgs; [
          fuse3.dev
          openssl.dev
          pkg-config
          s2n-tls
        ];

        runtimeDeps = with pkgs; [
          fuse3
          openssl
          sqlite
        ];
      in {
        packages.default =
          naersk'.buildPackage
          (env
          // {
            src = ./.;
            # release = false;
            nativeBuildInputs = (buildDeps ++ (with pkgs; [
              makeWrapper
            ]));
            buildInputs = runtimeDeps;
            postInstall = ''
              wrapProgram $out/bin/xkcd_fuse \
                --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath (with pkgs; [fuse3 openssl])}
            '';
          });

        formatter = pkgs.alejandra;

        devShells.default = with pkgs;
          mkShell (env // {
            buildInputs = [
              toolchain
              cargo-expand
            ] ++ buildDeps ++ runtimeDeps;

            RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
          });
      };
    };
}
