{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
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
      in {
        packages = {
          inherit ivyterm;
          default = ivyterm;
        };
      }
    );
}
