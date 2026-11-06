# caffiend

Watches a coffee roaster's webshop (DAK Coffee Roasters by default) and sends a message when a
coffee is new, and optionally when a sold-out one is back in stock (`--restocks`, off by default).
It reads the shop's public WooCommerce Store API, remembers what it has seen in `state.json`, and
notifies on the difference. The first run only remembers, so it never announces the whole shop.

A message carries what the shop says about the coffee: origin, region, varietal, process, tasting
notes, flavour tags, price, sizes and the link.

## Run it

    nix-shell --run 'cargo run -- --once --state-dir ./local'      # logs what it would send
    cargo test && cargo clippy --all-targets -- -D warnings

Notifiers (either or both; with neither it only logs):

- **Telegram:** `--telegram-token-file` (a bot token from @BotFather) and `--telegram-chat-id`
  (your user id; press Start on the bot first). The chat id is not a secret, the token is.
- **ntfy:** `--ntfy-topic-file` and optionally `--ntfy-server`. On a public server the topic is the
  only secret, so make it long and random.

`--send-test` sends one test message through them and exits. A new channel is a new variant of
`Notifier` in `src/notify.rs`.

## NixOS

The repository is a plain (non-flake) Nix project. Pin it, import the module, enable the service.

    # npins add github olistrik caffiend
    let
      caffiend = import sources.caffiend { inherit (sources) nixpkgs; };
    in
    {
      imports = [ caffiend.nixosModules.default ];

      services.caffiend = {
        enable = true;
        telegram = {
          enable = true;
          tokenFile = "/run/secrets/caffiend-telegram-token"; # any secret manager; not the Nix store
          chatId = "123456789";
        };
        # restocks = true;                  # also announce sold-out coffees coming back
        # categories = [ "filter" ];        # default: filter and espresso; [] means everything
        # interval = 600;                   # seconds between polls
      };
    }

The token file must be readable by the `caffiend` user (or whatever `user` is set to). For ntfy,
use `services.caffiend.ntfy = { enable = true; topicFile = ...; }` instead, or as well.

State lives in `services.caffiend.dataDir` (`/var/lib/caffiend`). Keep it across reboots: without
it, every restart is a silent first run and misses whatever appeared while the service was down.
On a setup that wipes the root filesystem on boot, add the directory to your persistence list.
