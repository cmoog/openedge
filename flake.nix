{
  description = "deno-edge: A serverless edge runtime for JavaScript, built with Deno.";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        naersk' = pkgs.callPackage naersk { };
        librustUrl = "https://github.com/denoland/rusty_v8/releases/download/v0.51.0/librusty_v8_release_x86_64-unknown-linux-gnu.a";
        prebuiltRustyV8 = builtins.fetchurl {
          url = librustUrl;
          sha256 = "0hpasrmk14wlqryaan1jsdn61x4s0hdanq5kas7x7kwxg00ap89k";
        };
        prebuiltRustyV8Sum = pkgs.writeText "rusty_v8_release_url" librustUrl;
      in
      {
        packages = {
          # TODO: fix. Broken due to libffi-sys build script needing write perms to source dir.
          # default = naersk'.buildPackage {
          #   src = ./.;
          #   buildInputs = [ pkgs.coreutils ];
          #   preBuild = ''
          #     mkdir -p /build/dummy-src/target/release/gn_out/obj
          #     cp ${prebuiltRustyV8} /build/dummy-src/target/release/gn_out/obj/librusty_v8.a
          #     cp ${prebuiltRustyV8Sum} /build/dummy-src/target/release/gn_out/obj/librusty_v8.sum
          #   '';
          # };
        };
        formatter = pkgs.nixpkgs-fmt;
        devShells.default = with pkgs; mkShell {
          packages = [
            cargo
            deno
            flyctl
            rust-analyzer
            rustc
            rustfmt
          ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      }
    );
}
