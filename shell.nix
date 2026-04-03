{ pkgs ? import <nixpkgs> {
    overlays = [
        (import <rust-overlay>)
    ];
}, ... }:
with pkgs;
let stdenv = llvmPackages.stdenv;
in stdenv.mkDerivation {
    name = "shell";
    nativeBuildInputs = [
        #gdb
        mold
        clang
        rust-bin.stable.latest.default
        rust-analyzer
        cargo-bloat

        pkg-config
        rustPlatform.bindgenHook
        #cargo-about
    ];
}
