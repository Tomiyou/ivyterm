{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crate2nix = {
      url = "github:nix-community/crate2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crate2nix, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };
        toolchain = fenix.packages.${system}.stable.defaultToolchain;
        cargo-nix = pkgs.callPackage (crate2nix.tools.${system}.generatedCargoNix {
          name = "ivyterm";
          src = ./.;
        }) {
          inherit pkgs;
          defaultCrateOverrides = pkgs.defaultCrateOverrides // {
            libadwaita-sys = attrs: {
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = [ pkgs.libadwaita ];
            };
            vte4-sys = attrs: {
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = [ pkgs.vte-gtk4 ];
            };
          };
        };
        ivyterm = cargo-nix.rootCrate.build;
      in {
        packages = {
          inherit ivyterm;
          default = ivyterm;
        };
        devShells.default = pkgs.mkShell {
          buildInputs = [ toolchain pkgs.libadwaita pkgs.vte-gtk4 pkgs.openssl pkgs.pkg-config ];
        };
      }
    );
}