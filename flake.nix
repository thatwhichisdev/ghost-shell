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
    in
    {
      devShells = forEachSystem (
        { system, pkgs }:
        let
          rustToolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./.config/rust-toolchain.toml;
            sha256 = "sha256-gh/xTkxKHL4eiRXzWv8KP7vfjSk61Iq48x47BEDFgfk=";
          };

          runtimeLibraries = with pkgs; [
            fontconfig
            freetype
            libxkbcommon
            vulkan-loader
            wayland
          ];
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

            shellHook = ''
              unset NIX_ENFORCE_PURITY
              exec nu --config ${./.config/config.nu}
            '';
          };
        }
      );

      formatter = forEachSystem ({ pkgs, ... }: pkgs.nixfmt-tree);
    };
}
