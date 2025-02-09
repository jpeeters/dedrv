# `dedrv`

`dedrv` ("de drivers" standing for "device drivers" or "delegated drivers") is a minimal library
for implementing device drivers on resource-contrained targets, like microcontrollers for instance.

The library provides the internal logic for decoupling device drivers from their public interface.
As a result, one can develop device drivers for many different targets but with the same interface.

Every device driver is also managed during its initialization and clean-up phase, offering
customization on these phases (e.g. ordering, priority, hook...).

## For developers

The developing environment is based on Nix developer shells, which offers a all-inclusive
developer experience, at least we hope so.

So as to start, just type the following.

If you have Nix with flakes:

```shell
nix develop
```

If you have `direnv` and Nix with flakes:

```shell
echo "use flake" >> .envrc
direnv allow
```

And if you do not want to type all this, there is a helper as a just rule:

```shell
just init direnv
```

Or:

```shell
nix run nixpkgs#just -- init direnv
```

if you do not have just installed on you Nix configuration.

## MSRV

The minimum supported Rust version is 1.76. `dedrv` is tested against the latest stable Rust
version and the MSRV.

## License

Licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.

### Contribution

Unless you explicitely state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any
additional terms or conditions.
