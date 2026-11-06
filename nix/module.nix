# The caffiend NixOS module: one service that polls WooCommerce shops and sends messages about
# new (and, opt-in, restocked) products. Secrets are file paths (strings), so they never enter the
# Nix store, for example `config.age.secrets.x.path`.
{ packages }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.caffiend;
  inherit (lib)
    mkEnableOption
    mkIf
    mkOption
    types
    ;
  json = pkgs.formats.json { };

  shopOptions = {
    options = {
      url = mkOption {
        type = types.str;
        example = "https://www.example-roaster.com";
        description = "The shop's address. Its WooCommerce Store API is read from there.";
      };
      name = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "What messages call the shop. Defaults to its hostname.";
      };
      categories = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [
          "filter"
          "espresso"
        ];
        description = "Only products in these category slugs are announced. Empty means all of them.";
      };
      attributes = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [
          "Origin"
          "Process"
          "Tasting notes"
        ];
        description = ''
          Which product attributes messages show, in this order. Empty means every attribute the
          shop provides. Names are the shop's own and match regardless of case.
        '';
      };
      restocks = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Also message when a sold-out product is back in stock. Off by default: new products are
          what matter, and stock flips often.
        '';
      };
    };
  };

  withoutNulls = lib.filterAttrs (_: v: v != null);
  configFile = json.generate "caffiend.json" (withoutNulls {
    interval_secs = cfg.interval;
    state_dir = cfg.dataDir;
    ntfy = mkIf' cfg.ntfy.enable {
      inherit (cfg.ntfy) server;
      topic_file = cfg.ntfy.topicFile;
    };
    telegram = mkIf' cfg.telegram.enable {
      token_file = cfg.telegram.tokenFile;
      chat_id = cfg.telegram.chatId;
    };
    shops = lib.mapAttrs (
      _: shop:
      withoutNulls {
        inherit (shop)
          url
          name
          categories
          attributes
          restocks
          ;
      }
    ) cfg.shops;
  });
  mkIf' = condition: value: if condition then value else null;
in
{
  options.services.caffiend = {
    enable = mkEnableOption "caffiend, the webshop watcher";

    package = mkOption {
      type = types.package;
      default = packages.caffiend;
      defaultText = "the caffiend package of this repository";
      description = "The package providing `caffiend`.";
    };

    shops = mkOption {
      type = types.attrsOf (types.submodule shopOptions);
      default = { };
      example = lib.literalExpression ''
        {
          dak = {
            url = "https://www.dakcoffeeroasters.com";
            name = "DAK";
            categories = [ "filter" "espresso" ];
            attributes = [ "Origin" "Process" "Tasting notes" ];
          };
        }
      '';
      description = ''
        The shops to watch, by a short key (lowercase letters, digits, `-` and `_`) that names the
        shop's state file and appears in logs.
      '';
    };

    interval = mkOption {
      type = types.ints.positive;
      default = 600;
      description = "Seconds between polls of the shops. Be kind to them.";
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
    assertions = [
      {
        assertion = cfg.shops != { };
        message = "services.caffiend: add at least one shop to `shops`.";
      }
      {
        assertion = lib.all (key: builtins.match "[a-z0-9_-]+" key != null) (lib.attrNames cfg.shops);
        message = "services.caffiend: shop keys may only hold a-z, 0-9, - and _.";
      }
    ];

    users.users = lib.mkIf (cfg.user == "caffiend") {
      caffiend = {
        isSystemUser = true;
        inherit (cfg) group;
      };
    };
    users.groups = lib.mkIf (cfg.group == "caffiend") { caffiend = { }; };

    systemd.tmpfiles.rules = [ "d ${cfg.dataDir} 0750 ${cfg.user} ${cfg.group} -" ];

    systemd.services.caffiend = {
      description = "Webshop watcher";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      restartTriggers = [ configFile ];
      environment.RUST_LOG = "info";
      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} --config ${configFile}";
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
