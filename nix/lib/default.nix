{ lib, ... }:

{
  # Override the `cleanCargoSource` from Crane library so it keeps the linker scripts.
  cleanCargoSource =
    someCraneLib: path:
    lib.fileset.toSource {
      root = path;
      fileset = lib.fileset.unions [
        # Default files from crane (rust and cargo files).
        (someCraneLib.fileset.commonCargoSources path)

        # Linker scripts that are needed for building (e.g. memory.x).
        (lib.fileset.fileFilter (file: file.hasExt "x") path)
      ];
    };

  # Override the `buildPackage` from Crane library so it can get a build profile.
  buildPackage =
    someCraneLib:
    {
      profile ? "dev",
    }:
    args: someCraneLib.buildPackage (args // { CARGO_PROFILE = "${profile}"; });

  # Set the default profile in the Crane library.
  cargoDerivationFor =
    { profile, ... }:
    someCraneLib:
    someCraneLib.overrideScope (
      _: prev: {
        # Override the `mkCargoDerivation` function so that every other functions
        # in Crane library that rely on it will have the default profile set.
        mkCargoDerivation = args: prev.mkCargoDerivation ({ CARGO_PROFILE = "${profile}"; } // args);
      }
    );

  # Set the rust toolchain in the Crane library.
  cargoRustToolchainFor =
    {
      extensions ? [ ],
      targets ? [ ],
      ...
    }:
    someCraneLib:
    someCraneLib.overrideToolchain (
      somePkgs:
      somePkgs.rust-bin.stable.latest.default.override {
        inherit extensions targets;
      }
    );
}
