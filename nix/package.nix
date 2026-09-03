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
  ];

  cargoPackageFlags = [
    "--package"
    "ghost-shell-cli"
    "--package"
    "ghost-shell-daemon"
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
  ];

  buildInputs = runtimeLibraries;

  cargoBuildFlags = cargoPackageFlags;
  cargoTestFlags = cargoPackageFlags;

  LIBPAMSYS_IMPL = "LinuxPam";

  postFixup = ''
    wrapProgram $out/bin/ghost-shell-daemon \
      --set XKB_CONFIG_ROOT "${xkeyboard_config}/share/X11/xkb" \
      --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath runtimeLibraries}"
  '';

  meta.platforms = lib.platforms.linux;
}
