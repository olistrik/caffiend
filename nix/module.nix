# The caffiend NixOS module: a service that polls the shop and sends messages about new and
# restocked coffees. Secrets are file paths (strings), so they never enter the Nix store, for
# example `config.age.secrets.x.path`.
{ packages }:
{
  config,
  lib,
  ...
}:

let
  cfg = config.services.caffiend;
  inherit (lib) mkEnableOption mkIf mkOption types;
in
{
  options.services.caffiend = {
    enable = mkEnableOption "caffiend, the coffee shop watcher";

    package = mkOption {
      type = types.package;
      default = packages.caffiend;
      defaultText = "the caffiend package of this repository";
      description = "The package providing `caffiend`.";
    };

    shopUrl = mkOption {
      type = types.str;
      default = "https://www.dakcoffeeroasters.com";
      description = "The WooCommerce shop to watch.";
    };

    categories = mkOption {
      type = types.listOf types.str;
      default = [
        "filter"
        "espresso"
      ];
      description = "Only products in these category slugs are announced. Empty means all of them.";
    };

    restocks = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Also message when a coffee that was sold out is back in stock. Off by default: new coffees
        are what matter, and stock flips often.
      '';
    };

    interval = mkOption {
      type = types.ints.positive;
      default = 600;
      description = "Seconds between polls. Be kind to the shop.";
    };

    user = mkOption {
      type = types.str;
      default = "caffiend";
      description = "The user the service runs as.";
    };

    group = mkOption {
      type = types.str;
      default = "caffiend";
      description = "The group the service runs as.";
    };

    dataDir = mkOption {
      type = types.path;
      default = "/var/lib/caffiend";
      description = ''
        Where the products seen so far are kept. Persist this: without it every restart is a first
        run, which stays silent but misses whatever appeared while the service was down.
      '';
    };

    ntfy = {
      enable = mkEnableOption "messages through ntfy";
      server = mkOption {
        type = types.str;
        default = "https://ntfy.sh";
        description = "The ntfy server, for example a self-hosted one.";
      };
      topicFile = mkOption {
        type = types.str;
        description = ''
          File holding the topic. On a public server the topic is the only secret, so make it long
          and random.
        '';
      };
    };

    telegram = {
      enable = mkEnableOption "messages through a Telegram bot";
      tokenFile = mkOption {
        type = types.str;
        description = "File holding the bot token from @BotFather.";
      };
      chatId = mkOption {
        type = types.str;
        description = "The chat to message: your user id, after you have started the bot.";
      };
    };
  };

  config = mkIf cfg.enable {
    users.users = lib.mkIf (cfg.user == "caffiend") {
      caffiend = {
        isSystemUser = true;
        inherit (cfg) group;
      };
    };
    users.groups = lib.mkIf (cfg.group == "caffiend") { caffiend = { }; };

    systemd.tmpfiles.rules = [ "d ${cfg.dataDir} 0750 ${cfg.user} ${cfg.group} -" ];

    systemd.services.caffiend = {
      description = "Coffee shop watcher";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      environment.RUST_LOG = "info";
      serviceConfig = {
        ExecStart = lib.concatStringsSep " " (
          [
            "${lib.getExe cfg.package}"
            "--shop-url ${lib.escapeShellArg cfg.shopUrl}"
            "--interval-secs ${toString cfg.interval}"
            "--state-dir ${cfg.dataDir}"
          ]
          ++ lib.optional cfg.restocks "--restocks"
          ++ map (c: "--category ${lib.escapeShellArg c}") cfg.categories
          ++ lib.optionals cfg.ntfy.enable [
            "--ntfy-server ${lib.escapeShellArg cfg.ntfy.server}"
            "--ntfy-topic-file ${lib.escapeShellArg cfg.ntfy.topicFile}"
          ]
          ++ lib.optionals cfg.telegram.enable [
            "--telegram-token-file ${lib.escapeShellArg cfg.telegram.tokenFile}"
            "--telegram-chat-id ${lib.escapeShellArg cfg.telegram.chatId}"
          ]
        );
        User = cfg.user;
        Group = cfg.group;
        Restart = "on-failure";
        RestartSec = 30;
        ReadWritePaths = [ cfg.dataDir ];
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
        ];
      };
    };
  };
}
