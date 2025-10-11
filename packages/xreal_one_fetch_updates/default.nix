{
  flake-utils,
  nixpkgs,
  uv2nix,
  pyproject-nix,
  pyproject-build-systems,
  ...
}@inputs:
flake-utils.lib.eachDefaultSystem (
  system:
  let
    pkgs = import nixpkgs {
      inherit system;
    };

    python = pkgs.python312;

    uvWorkspace = uv2nix.lib.workspace.loadWorkspace { workspaceRoot = ./.; };

    overlay = uvWorkspace.mkPyprojectOverlay {
      sourcePreference = "wheel";
    };

    pythonSet =
      (pkgs.callPackage pyproject-nix.build.packages {
        inherit python;
      }).overrideScope
        (
          pkgs.lib.composeManyExtensions [
            pyproject-build-systems.overlays.default
            overlay
          ]
        );
  in
  {
    packages.xreal_one_fetch_updates = (
      pythonSet.mkVirtualEnv "xreal_one_fetch_updates" uvWorkspace.deps.all
    );
  }
)
