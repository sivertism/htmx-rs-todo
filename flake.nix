{
  description = "Crappy to do list app";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, rust-overlay }:
    let 
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = nixpkgs.legacyPackages;
    in {

      packages = forAllSystems (system: 
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        default = pkgsFor.${system}.callPackage 
        pkgs.rustPlatform.buildRustPackage rec {
          pname = "htmx-rs-todo";
          version = "0.2.0";
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
          buildInputs = [
            pkgs.openssl
          ];
          nativeBuildInputs = [
            pkgs.pkg-config
          ];
        };
      });

      devShells = forAllSystems (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
        libPath = with pkgs; lib.makeLibraryPath [
          # load external libraries that you need in your rust project here
        ];
      in
      {
        default = pkgsFor.${system}.callPackage
          pkgs.mkShell rec {
            nativeBuildInputs = with pkgs; [
              # Build-time dependencies
              rustc
              cargo
              clang
              # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
              llvmPackages.bintools
              rustup
              rust-analyzer
              clippy
              pkg-config
              rustfmt
            ];
            buildInputs = with pkgs; [
              # Run-time dependencies
              openssl
              sqlite
              rustfmt
            ];
            RUSTC_VERSION = overrides.toolchain.channel;
            # https://github.com/rust-lang/rust-bindgen#environment-variables
            LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
            shellHook = ''
              export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
              export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
              '';
            # Add precompiled library to rustc search path
            RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
              # add libraries here (e.g. pkgs.libvmi)
            ]);
            LD_LIBRARY_PATH = libPath;
            # Add glibc, clang, glib, and other headers to bindgen search path
            BINDGEN_EXTRA_CLANG_ARGS =
            # Includes normal include path
            (builtins.map (a: ''-I"${a}/include"'') [
              # add dev libraries here (e.g. pkgs.libvmi.dev)
              pkgs.glibc.dev
            ])
            # Includes with special directory paths
            ++ [
              ''-I"${pkgs.llvmPackages_latest.libclang.lib}/lib/clang/${pkgs.llvmPackages_latest.libclang.version}/include"''
              ''-I"${pkgs.glib.dev}/include/glib-2.0"''
              ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
            ];
          };
      }
      );
    };
}
