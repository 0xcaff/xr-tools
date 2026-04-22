{
  flake-utils,
  nixpkgs,
  ...
}:
flake-utils.lib.eachDefaultSystem (
  system:
  let
    pkgs = import nixpkgs {
      inherit system;
    };
  in
  {
    packages.xreal_one_ctl = pkgs.rustPlatform.buildRustPackage {
      pname = "xreal_one_ctl";
      version = "0.1.0";

      src = ../..;
      cargoLock.lockFile = ../../Cargo.lock;
      buildAndTestSubdir = "packages/xreal_one_ctl";

      nativeBuildInputs = [
        pkgs.pkg-config
      ];

      buildInputs =
        pkgs.lib.optionals pkgs.stdenv.isLinux [
          pkgs.hidapi
          pkgs.udev
        ];
    };
  }
)
