{
  description = "A desktop shell built with GPUI, designed exclusively for Wayland and Niri. ";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
    }:
    let
      supportedSystems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      forEachSystem =
        function:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          function {
            inherit system;
            pkgs = import nixpkgs { inherit system; };
          }
        );

      rustToolchainFor =
        system:
        fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-gh/xTkxKHL4eiRXzWv8KP7vfjSk61Iq48x47BEDFgfk=";
        };

      runtimeLibrariesFor =
        pkgs: with pkgs; [
          fontconfig
          freetype
          libxkbcommon
          vulkan-loader
          wayland
          linux-pam
        ];
    in
    {
      packages = forEachSystem (
        { system, pkgs }:
        let
          rustToolchain = rustToolchainFor system;

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          runtimeLibraries = runtimeLibrariesFor pkgs;
        in
        rec {
          ghost-shell = rustPlatform.buildRustPackage {
            pname = "ghost-shell";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            strictDeps = true;
            buildType = "release";

            nativeBuildInputs = with pkgs; [
              pkg-config
              makeWrapper
            ];

            buildInputs = runtimeLibraries;

            cargoBuildFlags = [
              "--package"
              "ghost-shell-cli"
              "--package"
              "ghost-shell-daemon"
            ];

            cargoTestFlags = [
              "--package"
              "ghost-shell-cli"
              "--package"
              "ghost-shell-daemon"
            ];

            LIBPAMSYS_IMPL = "LinuxPam";

            postFixup = ''
              wrapProgram $out/bin/ghost-shell-daemon \
                --set XKB_CONFIG_ROOT "${pkgs.xkeyboard_config}/share/X11/xkb" \
                --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath runtimeLibraries}"
            '';
          };

          default = ghost-shell;
        }
      );

      devShells = forEachSystem (
        { system, pkgs }:
        let
          rustToolchain = rustToolchainFor system;
          runtimeLibraries = runtimeLibrariesFor pkgs;
        in
        {
          default = pkgs.mkShell {
            strictDeps = true;

            nativeBuildInputs = with pkgs; [
              rustToolchain
              pkg-config
              tombi
            ];

            buildInputs = runtimeLibraries;

            packages = with pkgs; [
              nushell
              starship
              nerd-fonts.jetbrains-mono
            ];

            LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibraries;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibraries;

            XKB_CONFIG_ROOT = "${pkgs.xkeyboard_config}/share/X11/xkb";
            STARSHIP_CONFIG = "${./.config/starship.toml}";

            LIBPAMSYS_IMPL = "LinuxPam";

            shellHook = ''
              unset NIX_ENFORCE_PURITY
              exec nu --config ${./.config/nushell.nu}
            '';
          };
        }
      );

      formatter = forEachSystem ({ pkgs, ... }: pkgs.nixfmt-tree);
    };
}
