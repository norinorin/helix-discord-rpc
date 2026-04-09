{
  description = "Helix Discord RPC Plugin (Steel)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "helix-discord-rpc";
          version = "0.1.0";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          buildType = "release";
          doCheck = false;

          installPhase = ''
            mkdir -p $out/lib
            mkdir -p $out/share/helix-discord-rpc

            find target -name "libhelix_discord_rpc.so" -o -name "libhelix_discord_rpc.dylib" -exec cp {} $out/lib/ \;

            cp *.scm $out/share/helix-discord-rpc/
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            rust-analyzer
            clippy
            steel
          ];

          shellHook = ''
            export STEEL_HOME=$PWD/.steel-env
            mkdir -p $STEEL_HOME/native

            if [ -f target/debug/libhelix_discord_rpc.so ]; then
               ln -sf $PWD/target/debug/libhelix_discord_rpc.so $STEEL_HOME/native/
            fi

            echo "Dev shell ready."
          '';
        };
      }
    );
}
