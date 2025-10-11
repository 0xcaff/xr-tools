{
  flake-utils,
  treefmt-nix,
  self,
  nixpkgs,
  ...
}:
(flake-utils.lib.eachDefaultSystem (
  system:
  let
    pkgs = import nixpkgs {
      inherit system;
    };

    config = {
      projectRootFile = "flake.nix";
      programs = {
        nixfmt.enable = true;
        black.enable = true;
      };
    };
    treefmtEval = treefmt-nix.lib.evalModule pkgs config;
  in
  {
    formatter = treefmtEval.config.build.wrapper;
    checks = {
      formatting = treefmtEval.config.build.check self;
    };
  }
))
