{
  pkgs,
  rustToolchain,
}:
let
  runtimeLibraries = with pkgs; [
    fontconfig
    freetype
    libxkbcommon
    vulkan-loader
    wayland
    linux-pam
  ];
in
pkgs.mkShell {
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
  STARSHIP_CONFIG = "${../.config/starship.toml}";

  LIBPAMSYS_IMPL = "LinuxPam";

  shellHook = ''
    unset NIX_ENFORCE_PURITY
    exec nu --config ${../.config/nushell.nu}
  '';
}
