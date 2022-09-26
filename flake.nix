{
  description = "openedge: A serverless edge runtime for JavaScript, built with Deno.";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      {
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
    )) // (
      # The derivations only support x86_64-linux for now...

      # The rusty_v8 build scripts were causing issues with the nix incremental build systems,
      # namely github:nix-community/naersk, so we will have to use rustPlatform.buildRustPackage for now. 

      # TODO: extend this derivation to more systems
      let
        system = "x86_64-linux";
        pkgs = nixpkgs.legacyPackages.${system};
        librustUrl = "https://github.com/denoland/rusty_v8/releases/download/v0.51.0/librusty_v8_release_x86_64-unknown-linux-gnu.a";
        prebuiltRustyV8 = builtins.fetchurl {
          url = librustUrl;
          sha256 = "0hpasrmk14wlqryaan1jsdn61x4s0hdanq5kas7x7kwxg00ap89k";
        };
        prebuiltRustyV8Sum = pkgs.writeText "rusty_v8_release_url" librustUrl;
      in
      {
        packages.${system} = rec {
          # info: https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md
          default = with pkgs; rustPlatform.buildRustPackage {
            pname = "openedge";
            version = "0.1.0";
            src = ./.;
            cargoHash = "sha256-Dx4eFEc0d4HiJd3wqNDMSxu6lsgadrHl0GKfP2d6e1I=";
            preBuild = ''
              mkdir -p ./target/x86_64-unknown-linux-gnu/release/gn_out/obj
              cp ${prebuiltRustyV8} ./target/x86_64-unknown-linux-gnu/release/gn_out/obj/librusty_v8.a
              cp ${prebuiltRustyV8Sum} ./target/x86_64-unknown-linux-gnu/release/gn_out/obj/librusty_v8.sum

              mkdir -p ./target/release/gn_out/obj
              cp ${prebuiltRustyV8} ./target/release/gn_out/obj/librusty_v8.a
              cp ${prebuiltRustyV8Sum} ./target/release/gn_out/obj/librusty_v8.sum
            '';
            meta = with lib; {
              description = "An open source serverless edge runtime for JavaScript.";
              homepage = "https://github.com/cmoog/openedge";
              license = licenses.mit;
            };
          };
          container = with pkgs; dockerTools.buildLayeredImage {
            name = "openedge";
            tag = self.shortRev or "dirty";
            config = {
              Cmd = [ "${default}/bin/openedge" ];
              ExposedPorts = {
                "8080/tcp" = { };
              };
            };
          };
        };
      }
    );
}
