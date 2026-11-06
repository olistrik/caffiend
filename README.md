# caffiend

Watches a coffee roaster's webshop (DAK Coffee Roasters by default) and sends a message when a
coffee is new, and optionally when a sold-out one is back in stock (`--restocks`, off by default). It reads the shop's public WooCommerce Store API, remembers what
it has seen in `state.json`, and notifies on the difference. The first run only remembers.

## Run it

    nix-shell --run 'cargo run -- --once --state-dir ./local'      # logs what it would send
    cargo test && cargo clippy --all-targets -- -D warnings

Notifiers (both may be on): ntfy (`--ntfy-topic-file`, optional `--ntfy-server`) and Telegram
(`--telegram-token-file` plus `--telegram-chat-id`). With neither it only logs. A new channel
(Signal through signal-cli, say) is a new variant of `Notifier` in `src/notify.rs`.

## Deploy on hestia

Pinned and imported like homewire, in `/persist/nixos`:

    npins add github <you> caffiend      # or git+ssh, as homewire is
    # projects.nix
    caffiend = import sources.caffiend { inherit (sources) nixpkgs; };

    # modules/hosts/hestia/caffiend.nix
    imports = [ my.projects.caffiend.nixosModules.default ];
    environment.persistence."/persist".directories = [ "/var/lib/caffiend" ];
    age.secrets.caffiend-ntfy-topic = { file = ...; owner = "caffiend"; };
    services.caffiend = {
      enable = true;
      ntfy = { enable = true; topicFile = config.age.secrets.caffiend-ntfy-topic.path; };
    };

Root is wiped on boot there, so `dataDir` must be persisted or each reboot is a silent first run.
