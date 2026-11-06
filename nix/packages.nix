# The caffiend binary. Built from the lock file, so the dependencies are the ones that were tested.
{ pkgs }:

let
  inherit (pkgs) lib;
  root = ../.;
  caffiend = pkgs.rustPlatform.buildRustPackage {
    pname = "caffiend";
    version = "0.1.0";
    src = lib.fileset.toSource {
      inherit root;
      fileset = lib.fileset.unions [
        (root + "/Cargo.toml")
        (root + "/Cargo.lock")
        (root + "/src")
        # The unit tests parse a recorded Store API response.
        (root + "/tests")
      ];
    };
    cargoLock.lockFile = root + "/Cargo.lock";
    meta.mainProgram = "caffiend";
  };
in
{
  inherit caffiend;
  default = caffiend;
}
