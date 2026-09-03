{
  description = "A desktop shell built with GPUI, designed exclusively for Wayland and Niri.";

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

      forEachSystem = nixpkgs.lib.genAttrs supportedSystems;

      pkgsFor = system: nixpkgs.legacyPackages.${system};

      rustToolchainFor =
        system:
        fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-gh/xTkxKHL4eiRXzWv8KP7vfjSk61Iq48x47BEDFgfk=";
        };
    in
    {
      packages = forEachSystem (
        system:
        let
          pkgs = pkgsFor system;
          rustToolchain = rustToolchainFor system;

          ghost-shell = pkgs.callPackage ./nix/package.nix {
            inherit rustToolchain;
          };
        in
        {
          inherit ghost-shell;
          default = ghost-shell;
        }
      );

      devShells = forEachSystem (
        system:
        let
          pkgs = pkgsFor system;
          rustToolchain = rustToolchainFor system;
        in
        {
          default = import ./nix/dev-shell.nix {
            inherit pkgs rustToolchain;
          };
        }
      );

      formatter = forEachSystem (system: (pkgsFor system).nixfmt-tree);
    };
}
