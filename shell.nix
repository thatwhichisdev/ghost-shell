let
  nixpkgs = builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz";
  };

  fenixSrc = builtins.fetchTarball {
    url = "https://github.com/nix-community/fenix/archive/main.tar.gz";
  };

  pkgs = import nixpkgs {
    overlays = [
      (import "${fenixSrc}/overlay.nix")
    ];
  };

  rustToolchain = pkgs.fenix.fromToolchainFile {
    file = ./rust-toolchain.toml;
    sha256 = "sha256-NNcHR6rM11SWJaf1QAkZMoJHUqY4Y635aZVvQclTKG8=";
  };

  nativeLibraries = with pkgs; [
    fontconfig
    freetype
    libxkbcommon
    vulkan-loader
    wayland
  ];
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    pkg-config
  ];

  buildInputs = nativeLibraries;

  packages = with pkgs; [
    nushell
    starship
    nerd-fonts.jetbrains-mono
  ];

  # Used when linking native dependencies.
  LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeLibraries;

  # Used by GPUI/wayland-rs dlopen calls at runtime.
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeLibraries;

  # Needed later when xkbcommon loads keyboard definitions.
  XKB_CONFIG_ROOT = "${pkgs.xkeyboard_config}/share/X11/xkb";

  STARSHIP_CONFIG = "${./.starship/starship.toml}";

  shellHook = ''
    unset NIX_ENFORCE_PURITY
    exec nu --config ${./.nu/config.nu}
  '';
}
