{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.ghost-shell;

  tomlFormat = pkgs.formats.toml { };

  configFile = tomlFormat.generate "ghost-shell-config.toml" cfg.settings;
in
{
  options.programs.ghost-shell = {
    enable = lib.mkEnableOption "Ghost desktop shell";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.ghost-shell;
      description = "The Ghost package to install and run.";
    };

    settings = lib.mkOption {
      inherit (tomlFormat) type;

      default = { };

      example = lib.literalExpression ''
        {
          general = {
            font_family = "Berkeley Mono";
            font_size = 13;
          };

          wallpaper = {
            path = "/some/wallpaper.gif";
          };

          bar.primary = {
            output = "DP-1";
            height = 27;
            exclusive_zone = 27;
            primary = true;
          };
        }
      '';

      description = ''
        Ghost configuration written to
        `$XDG_CONFIG_HOME/ghost-shell/config.toml`.
      '';
    };

    systemd.enable = lib.mkEnableOption "ghost-shell systemd user service";
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."ghost-shell/config.toml".source = configFile;

    systemd.user.services.ghost-shell = lib.mkIf cfg.systemd.enable {
      Unit = {
        Description = "A desktop shell built exclusively for the Niri Wayland compositor";

        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        Requisite = [ "graphical-session.target" ];

        ConditionEnvironment = "NIRI_SOCKET";

        X-Restart-Triggers = [ "${configFile}" ];
      };

      Service = {
        ExecStart = lib.getExe' cfg.package "ghost-shell-daemon";
        Restart = "on-failure";
      };

      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}
