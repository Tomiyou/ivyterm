{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk {};
        nativeBuildInputs = [ pkgs.pkg-config pkgs.gtk4 pkgs.libadwaita pkgs.openssl pkgs.vte-gtk4 ];
        ivyterm = naersk'.buildPackage {
          inherit nativeBuildInputs;
          src = ./.;
        };
      in {
        packages = {
          inherit ivyterm;
          default = ivyterm;
        };
      }
    );
}
