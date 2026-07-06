{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
    	url = "github:oxalica/rust-overlay";
    	inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-compat = {
      url = github:edolstra/flake-compat;
      flake = true;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {inherit system overlays;};
        manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
        commonBuildInputs = with pkgs; [
          pkg-config
          openssl.dev
          libclang.lib
					systemd
					zstd
       ];
        runtimeDependencies = with pkgs; [
        ];
        package = pkgs.rustPlatform.buildRustPackage {
					name = manifest.name;
					pversion = manifest.version;

					src = pkgs.lib.cleanSource ./.;
					cargoLock = {
						lockFile = ./Cargo.lock;
					};
					doCheck = true;

					nativeBuildInputs = [
						pkgs.autoPatchelfHook
						pkgs.pkg-config
					];

					LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
					ZSTD_SYS_USE_PKG_CONFIG = true;

					runtimeDependencies = runtimeDependencies;

					buildInputs = with pkgs; [
					] ++ commonBuildInputs;

          buildFeatures = [
          	"socket"
          	"systemd-socket"
          ];

					meta = {
#						description = "";
#						license = pkgs.lib.licenses.unfree; #TODO: Re-Mark this as unfree, once you can actually use flakes with unfree packages
						platforms = pkgs.lib.platforms.linux ++ pkgs.lib.platforms.windows ++ pkgs.lib.platforms.darwin;
						mainProgram = "NeoLuma-Site";
					};
				};
      in
      {
        defaultPackage = package;
        packages = {
        	"default" = package;
        	"server" = package;
        };

        defaultApp = utils.lib.mkApp {
          drv = self.defaultPackage."${system}";
        };

        devShell = with pkgs; mkShell {
          buildInputs = [
            #cargo
            cargo-insta
            pre-commit
            sqlx-cli
            #rust-analyzer
            #rustPackages.clippy
            #rustc
            #rustfmt
            tokei
          ] ++ commonBuildInputs;
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          LD_LIBRARY_PATH = lib.makeLibraryPath commonBuildInputs;
          GIT_EXTERNAL_DIFF = "${difftastic}/bin/difft";
					RUST_BACKTRACE= "1";
					RUST_LIB_BACKTRACE = "1";
					ZSTD_SYS_USE_PKG_CONFIG = true;
        };
      });
}
