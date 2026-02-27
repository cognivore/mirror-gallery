{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.mirror-gallery;
  homeDir = config.home.homeDirectory;
  expandHome = s: builtins.replaceStrings [ "$HOME" ] [ homeDir ] s;
in
{
  options.programs.mirror-gallery = {
    enable = lib.mkEnableOption "mirror-gallery GitHub repository mirroring tool";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.mirror-gallery;
      defaultText = lib.literalExpression "pkgs.mirror-gallery";
      description = "The mirror-gallery package to install.";
    };

    rootDir = lib.mkOption {
      type = lib.types.str;
      default = "$HOME/Mirrors/Github";
      description = "Root directory for mirrored repositories.";
    };

    owners = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "NixOS" "cognivore" ];
      description = "GitHub owners (organisations or users) to mirror.";
    };

    timer = {
      enable = lib.mkEnableOption "periodic mirror-gallery sync (Linux: systemd timer, macOS: launchd agent)";

      intervalSec = lib.mkOption {
        type = lib.types.int;
        default = 86400;
        example = 3600;
        description = "Sync interval in seconds (default: daily). Used as systemd OnUnitActiveSec and launchd StartInterval.";
      };
    };
  };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      {
        home.packages = [ cfg.package ];
        home.sessionVariables.MIRROR_GALLERY_ROOT = cfg.rootDir;
      }

      (lib.mkIf (cfg.timer.enable && cfg.owners != [ ]) (
        lib.mkMerge [
          (lib.mkIf pkgs.stdenv.isLinux {
            systemd.user.services.mirror-gallery = {
              Unit.Description = "Mirror GitHub repositories";
              Service = {
                Type = "oneshot";
                ExecStart = "${lib.getExe cfg.package} ${lib.escapeShellArgs cfg.owners}";
                Environment = [
                  "MIRROR_GALLERY_ROOT=${expandHome cfg.rootDir}"
                  "HOME=${homeDir}"
                ];
              };
            };
            systemd.user.timers.mirror-gallery = {
              Unit.Description = "Periodic GitHub mirror sync";
              Timer = {
                OnBootSec = "5m";
                OnUnitActiveSec = toString cfg.timer.intervalSec;
                Persistent = true;
              };
              Install.WantedBy = [ "timers.target" ];
            };
          })

          (lib.mkIf pkgs.stdenv.isDarwin {
            launchd.agents.mirror-gallery = {
              enable = true;
              config = {
                ProgramArguments = [ "${lib.getExe cfg.package}" ] ++ cfg.owners;
                EnvironmentVariables = {
                  MIRROR_GALLERY_ROOT = expandHome cfg.rootDir;
                  HOME = homeDir;
                };
                StartInterval = cfg.timer.intervalSec;
                StandardOutPath = "${homeDir}/Library/Logs/mirror-gallery.log";
                StandardErrorPath = "${homeDir}/Library/Logs/mirror-gallery.log";
              };
            };
          })
        ]
      ))
    ]
  );
}
