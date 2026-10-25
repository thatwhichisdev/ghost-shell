{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.ghost-shell-greeter;
  tomlFormat = pkgs.formats.toml { };
  greeterConfig = tomlFormat.generate "ghost-shell-greeter.toml" cfg.settings;
  launchGreeter = pkgs.writeShellScript "ghost-shell-greeter-session" ''
    ${lib.getExe' cfg.package "ghost-shell-greeter"} --config ${greeterConfig}
    result=$?
    ${lib.getExe' pkgs.sway "swaymsg"} exit
    exit "$result"
  '';
  compositorConfig = pkgs.writeText "ghost-shell-greeter-sway.conf" ''
    xwayland disable
    output * bg #000000 solid_color
    exec ${launchGreeter}
  '';
in
{
  options.services.ghost-shell-greeter = {
    enable = lib.mkEnableOption "Ghost graphical greetd greeter";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.ghost-shell;
      description = "Package containing ghost-shell-greeter.";
    };

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = { };
      description = "System greeter configuration. Wallpaper paths must be readable by greeter before login.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.ghost-shell-greeter.settings = {
      session.command = lib.mkDefault [ "${pkgs.niri}/bin/niri-session" ];
      session.environment = lib.mkDefault [
        "XDG_SESSION_TYPE=wayland"
        "XDG_CURRENT_DESKTOP=niri"
        "XDG_SESSION_DESKTOP=niri"
      ];
    };

    environment.etc."ghost-shell/greeter.toml".source = greeterConfig;
    hardware.graphics.enable = true;
    services.greetd = {
      enable = true;
      useTextGreeter = false;
      settings.default_session = {
        user = "greeter";
        command = "${lib.getExe pkgs.sway} --config ${compositorConfig}";
      };
    };
  };
}
