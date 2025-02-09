{
  description = "A minimal library for implementing device drivers on resource-constrained targets";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";

    crane.url = "github:ipetkov/crane";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      treefmt-nix,
      ...
    }@inputs:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import inputs.rust-overlay) ];
        pkgs = import nixpkgs { inherit overlays system; };

        # Import helpers from the flake (e.g. cleanCargoSource).
        helpers = import ./nix/lib {
          inherit (pkgs) lib;
        };

        # Make the Crane lib instance.
        craneLib = pkgs.lib.foldl (acc: f: f acc) (inputs.crane.mkLib pkgs) [
          # Set the default profile to `dev`.
          (helpers.cargoDerivationFor { profile = "dev"; })

          # Customize the Rust toolchain for the target and crate.
          (helpers.cargoRustToolchainFor {
            # All needed extensions for editor completion, build and test.
            extensions = [
              "clippy"
              "llvm-tools"
              "rust-analyzer"
              "rust-src"
              "rust-std"
              "rustfmt"
            ];

            # The target we build for.
            targets = [ "thumbv8m.main-none-eabihf" ];
          })
        ];

        # Apply Crane library to source helper.
        cleanCargoSource = helpers.cleanCargoSource craneLib;

        # Apply Crane library to build helper (dev mode).
        buildPackage = helpers.buildPackage craneLib { };

        # Apply Crane library to build helper (dev mode).
        buildReleasePackage = helpers.buildPackage craneLib { profile = "release"; };
      in
      {
        # Declare the formatters to be used with `nix fmt`.
        formatter = treefmt-nix.lib.mkWrapper pkgs {
          # Where to look for the root of the sources.
          projectRootFile = "flake.nix";

          # What formatters are enabled.
          programs.mdformat.enable = true;
          programs.just.enable = true;
          programs.nixfmt.enable = true;
          programs.rustfmt.enable = true;
          programs.taplo.enable = true;

          # Formatter settings.
          settings.formatter.just.includes = [
            "Justfile"
            "**/*.just"
          ];
          settings.formatter.mdformat.includes = [ "**/*.md" ];
        };

        # Declare the developer shell with some build and utility packages.
        devShells.default = craneLib.devShell {
          # Additional environment variables here (e.g. CUSTOM_VAR = ...).

          # Extra input packages.
          packages = with pkgs; [
            bacon
            binsider
            cargo-audit
            cargo-binutils
            cargo-deny
            cargo-nextest
            just
            minicom
          ];
        };
      }
    );
}
