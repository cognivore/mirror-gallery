{
  description = "Mirror Gallery — clone and maintain mirrors of all GitHub repositories for given owners";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    claude-code.url = "github:sadjow/claude-code-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      claude-code,
      ...
    }:
    {
      overlays.default = final: _prev: {
        mirror-gallery = import ./nix/package.nix { pkgs = final; };
      };

      homeManagerModules.default = import ./nix/home-manager.nix;
      homeManagerModules.mirror-gallery = self.homeManagerModules.default;
    }
    //
      flake-utils.lib.eachSystem
        [
          "x86_64-linux"
          "aarch64-linux"
          "aarch64-darwin"
          "x86_64-darwin"
        ]
        (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              config.allowUnfree = true;
              overlays = [ self.overlays.default ];
            };
          in
          {
            packages = {
              default = pkgs.mirror-gallery;
              mirror-gallery = pkgs.mirror-gallery;
            };

            devShells.default = pkgs.mkShell {
              buildInputs = [
                pkgs.mirror-gallery
                claude-code.packages.${system}.default

                # Rust toolchain (same pattern as grim-monolith)
                pkgs.rust-script
                pkgs.rustc
                pkgs.cargo
                pkgs.rustfmt
                pkgs.clippy
                pkgs.rust-analyzer

                # Runtime deps available for hacking
                pkgs.gh
                pkgs.git
                pkgs.curl
                pkgs.jq
              ];

              RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

            };
          }
        );
}
