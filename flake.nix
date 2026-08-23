{
  description = "RelayKit development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          clippy
          pkg-config
          rust-analyzer
          rustc
          rustfmt
        ];
      };

      formatter.${system} = pkgs.nixfmt;
    };
}
