{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        # cargo-nix = import ./Cargo.nix { inherit pkgs; };
        cargo-nix = import ./Cargo.nix {
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
        toolchain = pkgs.rust-bin.stable.latest.default;
      in {
        packages = {
          inherit ivyterm;
          default = ivyterm;
        };
        devShells.default = pkgs.mkShell {
          buildInputs = [ toolchain ];
        };
      }
    );
}
