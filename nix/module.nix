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
  applicationNames = [
    "bar"
    "lockscreen"
  ];
  applications = lib.filterAttrs (_: application: application.enable) (
    lib.genAttrs applicationNames (name: cfg.applications.${name})
  );
  configFile = tomlFormat.generate "ghost-shell-config.toml" cfg.settings;
in
{
  imports =
    map
      (
        name:
        lib.mkRemovedOptionModule [ "programs" "ghost-shell" "applications" name "settings" ]
          "Move these sections into programs.ghost-shell.settings; shared theme and general settings are defined once."
      )
      (
        applicationNames
        ++ [
          "launcher"
          "finder"
        ]
      )
    ++
      map
        (
          name:
          lib.mkRemovedOptionModule [ "programs" "ghost-shell" "applications" name "enable" ]
            "Launcher and finder run on demand. Remove this service option and bind ghost-shell-${name} directly in Niri."
        )
        [
          "launcher"
          "finder"
        ];

  options.programs.ghost-shell = {
    enable = lib.mkEnableOption "Ghost desktop applications";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.ghost-shell;
      description = "The Ghost applications and CLI package to install.";
    };

    applications = lib.genAttrs applicationNames (name: {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Enable the Ghost ${name} application.";
      };
    });

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = { };
      description = "Shared desktop configuration written to $XDG_CONFIG_HOME/ghost-shell/config.toml.";
    };

    systemd.enable = lib.mkEnableOption "independent Ghost systemd user services";
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."ghost-shell/config.toml".source = configFile;

    systemd.user.services = lib.mkIf cfg.systemd.enable (
      lib.mapAttrs' (
        name: _:
        lib.nameValuePair "ghost-shell-${name}" {
          Unit = {
            Description = "Ghost ${name}";
            PartOf = [ "graphical-session.target" ];
            After = [ "graphical-session.target" ];
            Requisite = [ "graphical-session.target" ];
            ConditionEnvironment = "WAYLAND_DISPLAY";
            X-Restart-Triggers = lib.optionals (name == "lockscreen") [ "${configFile}" ];
          };
          Service = {
            ExecStart = lib.getExe' cfg.package "ghost-shell-${name}";
            Restart = if name == "lockscreen" then "no" else "on-failure";
          }
          // lib.optionalAttrs (name == "lockscreen") {
            # SIGTERM is handled by the lockscreen and exit waits for authentication.
            KillMode = "mixed";
            TimeoutStopSec = "infinity";
            SendSIGKILL = false;
          };
          Install.WantedBy = [ "graphical-session.target" ];
        }
      ) applications
    );
  };
}
