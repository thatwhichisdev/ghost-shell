{ inputs, config, ... }:
{
  imports = [ inputs.ghost-shell.nixosModules.greeter ];

  programs.niri.enable = true;

  services.ghost-shell-greeter = {
    enable = true;
    settings = {
      session.command = [ "${config.programs.niri.package}/bin/niri-session" ];
      general.font_family = "monospace";
      general.font_size = 13.0;
      wallpaper.bg = 658448;
      # Copy a wallpaper next to this file and uncomment to include it in the store.
      # wallpaper.path = "${./wallpaper.jpg}";
      theme.mode = "dark";
    };
  };
}
