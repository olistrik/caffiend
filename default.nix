# Entry point for building and deploying caffiend without flakes: the package and the NixOS module.
# A host imports it the way it imports homewire:
#   caffiend = import sources.caffiend { inherit (sources) nixpkgs; };
#   imports = [ caffiend.nixosModules.default ];
args@{
  system ? builtins.currentSystem,
  ...
}:

let
  sourceOverrides = builtins.removeAttrs args [ "system" ];
  sources = (import ./npins) // sourceOverrides;
  pkgs = import sources.nixpkgs { inherit system; };

  packages = import ./nix/packages.nix { inherit pkgs; };
  nixosModule = import ./nix/module.nix { inherit packages; };
in
{
  inherit packages nixosModule;
  nixosModules.default = nixosModule;
  tests.eval = import ./nix/tests/eval.nix;
}
