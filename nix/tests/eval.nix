# Evaluates the module in a minimal NixOS system, so `nix-build -A tests.eval` catches option
# mistakes without a VM. Builds the unit file the service would get.
let
  sources = import ../../npins;
  caffiend = import ../../default.nix { };
  system = import (sources.nixpkgs + "/nixos") {
    configuration = {
      imports = [ caffiend.nixosModules.default ];
      boot.loader.grub.enable = false;
      fileSystems."/" = {
        device = "none";
        fsType = "tmpfs";
      };
      system.stateVersion = "25.11";
      services.caffiend = {
        enable = true;
        shops = {
          dak = {
            url = "https://www.dakcoffeeroasters.com";
            name = "DAK";
            categories = [ "filter" "espresso" ];
            attributes = [ "Origin" "Tasting notes" ];
          };
          other.url = "https://shop.example.com";
        };
        ntfy = {
          enable = true;
          topicFile = "/run/secrets/caffiend-ntfy-topic";
        };
        telegram = {
          enable = true;
          tokenFile = "/run/secrets/caffiend-telegram-token";
          chatId = "123456";
        };
      };
    };
  };
in
system.config.systemd.units."caffiend.service".unit
