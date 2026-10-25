{
  lib,
  makeRustPlatform,
  rustToolchain,

  pkg-config,
  makeWrapper,

  fontconfig,
  freetype,
  libxkbcommon,
  vulkan-loader,
  wayland,
  linux-pam,
  pipewire,
  xkeyboard_config,
}:
let
  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  runtimeLibraries = [
    fontconfig
    freetype
    libxkbcommon
    vulkan-loader
    wayland
    linux-pam
    pipewire
  ];

  cargoPackageFlags = [
    "--package"
    "ghost-shell-cli"
    "--package"
    "ghost-shell-bar"
    "--package"
    "ghost-shell-launcher"
    "--package"
    "ghost-shell-finder"
    "--package"
    "ghost-shell-lockscreen"
    "--package"
    "ghost-shell-greeter"
  ];
in
rustPlatform.buildRustPackage {
  pname = "ghost-shell";
  version = "0.1.0";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  strictDeps = true;
  buildType = "release";

  nativeBuildInputs = [
    pkg-config
    makeWrapper
    rustPlatform.bindgenHook
  ];

  buildInputs = runtimeLibraries;

  cargoBuildFlags = cargoPackageFlags;
  cargoTestFlags = cargoPackageFlags;

  LIBPAMSYS_IMPL = "LinuxPam";

  postFixup = ''
    for application in ghost-shell-bar ghost-shell-launcher ghost-shell-finder ghost-shell-lockscreen ghost-shell-greeter; do
      wrapProgram $out/bin/$application \
        --set XKB_CONFIG_ROOT "${xkeyboard_config}/share/X11/xkb" \
        --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath runtimeLibraries}"
    done
  '';

  meta.platforms = lib.platforms.linux;
}
