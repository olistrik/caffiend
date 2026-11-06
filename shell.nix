let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
in
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc
    clippy
    rustfmt
    rust-analyzer
    cargo-watch
    npins
    # reqwest's TLS is rustls, but cc and pkg-config keep any -sys crate happy.
    gcc
    pkg-config
  ];
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
}
