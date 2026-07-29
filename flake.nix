{
  description = "Vidya — GNOME/HIG-inspired theme layer for egui";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

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
      pkgsFor = system: nixpkgs.legacyPackages.${system};

      eguiLibs =
        pkgs:
        with pkgs;
        [
          libxkbcommon
          libGL
          vulkan-loader
        ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
        ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          inherit (pkgs) lib;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "vidya";
            version = "0.1.0";
            src = lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "--lib" ];
            dontCargoInstall = true;
            postInstall = ''
              mkdir -p $out/share/vidya
              cp Cargo.toml Cargo.lock $out/share/vidya/
              cp -a src $out/share/vidya/
              cp README.md $out/share/vidya/ 2>/dev/null || true
            '';
            meta = {
              description = "Vidya — theme layer for egui";
              homepage = "https://tangled.org/nandi.uk/vidya";
              license = lib.licenses.mit;
            };
          };
        }
      );

      # Thin shell: just + adb + GL libs for host UI.
      # Use your normal rustup toolchain (with x86_64-linux-android for APK).
      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          libs = eguiLibs pkgs;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              just
              android-tools # adb
            ];
            buildInputs = libs;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
            shellHook = ''
              export PATH="$HOME/.cargo/bin:$PATH"
              export ANDROID_NDK_HOME="''${ANDROID_NDK_HOME:-$HOME/.local/share/android-ndk-r29}"
              export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
              export ANDROID_HOME="''${ANDROID_HOME:-$HOME/.local/share/android-sdk}"
              export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$ANDROID_HOME/platform-tools:$PATH"
              export CC_x86_64_linux_android="''${CC_x86_64_linux_android:-x86_64-linux-android28-clang}"
              export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"
              export AR_x86_64_linux_android="''${AR_x86_64_linux_android:-llvm-ar}"
              echo "vidya — just waydroid | just host | just shots"
            '';
          };
        }
      );
    };
}
