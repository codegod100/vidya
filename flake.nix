{
  description = "Vidya — GNOME/HIG-inspired theme layer for egui";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          inherit (pkgs) lib;
          vidya = pkgs.rustPlatform.buildRustPackage {
            pname = "vidya";
            version = "0.1.0";
            src = lib.cleanSource ./.;

            cargoLock.lockFile = ./Cargo.lock;

            # Pure library: cargo still produces rlib under target/
            # Install as a src + rlib bundle for dependents / docs.
            postInstall = ''
              mkdir -p $out/lib $out/share/vidya
              # rlib + deps from cargo
              if compgen -G "target/release/libvidya*.rlib" > /dev/null; then
                cp -a target/release/libvidya*.rlib $out/lib/ || true
              fi
              # Ship sources for path-style re-use
              cp Cargo.toml Cargo.lock $out/share/vidya/
              cp -a src $out/share/vidya/
              cp README.md $out/share/vidya/ 2>/dev/null || true
            '';

            meta = {
              description = "Vidya — theme layer for egui";
              homepage = "https://tangled.org/nandi.uk/global";
              license = lib.licenses.mit;
              platforms = systems;
            };
          };
        in
        {
          default = vidya;
          inherit vidya;
        }
      );

      overlays.default = final: _prev: {
        vidya = self.packages.${final.system}.vidya;
      };

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
              rustfmt
              clippy
            ];
          };
        }
      );
    };
}
