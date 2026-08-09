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
          libs = eguiLibs pkgs;
          libPath = lib.makeLibraryPath libs;

          # `cargo run` launcher: rustup's cargo + nix-provided GL/Wayland libs.
          # Prefers a live checkout (cwd); falls back to the flake source in the store.
          demoRunner = pkgs.writeShellApplication {
            name = "vidya-demo";
            text = ''
              export LD_LIBRARY_PATH="${libPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
              # Same cargo discovery as devShell / just host
              export PATH="''${HOME}/.cargo/bin:$PATH"
              if ! command -v cargo >/dev/null 2>&1; then
                _vidya_tc=""
                if [ -f "''${HOME}/.rustup/settings.toml" ]; then
                  _vidya_def=$(sed -n 's/^default_toolchain = "\(.*\)"/\1/p' "''${HOME}/.rustup/settings.toml" | head -1)
                  if [ -n "''${_vidya_def}" ] && [ -x "''${HOME}/.rustup/toolchains/''${_vidya_def}/bin/cargo" ]; then
                    _vidya_tc="''${HOME}/.rustup/toolchains/''${_vidya_def}/bin"
                  fi
                fi
                if [ -z "''${_vidya_tc}" ]; then
                  for _vidya_c in \
                    "''${HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin" \
                    "''${HOME}/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin" \
                    "''${HOME}/.rustup/toolchains/"*/bin; do
                    if [ -x "''${_vidya_c}/cargo" ]; then
                      _vidya_tc="''${_vidya_c}"
                      break
                    fi
                  done
                fi
                if [ -n "''${_vidya_tc}" ]; then
                  export PATH="''${_vidya_tc}:$PATH"
                fi
                unset _vidya_tc _vidya_def _vidya_c
              fi

              if ! command -v cargo >/dev/null 2>&1; then
                echo "vidya-demo: cargo not found — install rustup or put cargo on PATH" >&2
                exit 127
              fi

              run_host() {
                local root=$1
                shift
                exec cargo run --manifest-path "$root/host/Cargo.toml" -- "$@"
              }

              # nix run keeps the caller's cwd → live edit/rebuild against the tree
              if [ -f host/Cargo.toml ]; then
                run_host "$PWD" "$@"
              fi

              # App launcher / nix run from another directory → flake source (read-only)
              flake_src=${lib.escapeShellArg (toString self)}
              if [ -f "$flake_src/host/Cargo.toml" ]; then
                export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-''${XDG_CACHE_HOME:-$HOME/.cache}/vidya/cargo-target}"
                run_host "$flake_src" "$@"
              fi

              echo "vidya-demo: host/Cargo.toml not found" >&2
              echo "  run from the vidya checkout, or: nix run path:." >&2
              exit 1
            '';
          };
        in
        rec {
          # Theme library sources + rlib (for consumers / inspection).
          vidya = pkgs.rustPlatform.buildRustPackage {
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

          # Desktop showcase: cargo-run wrapper + .desktop entry.
          demo = pkgs.symlinkJoin {
            name = "vidya-demo";
            paths = [ demoRunner ];
            postBuild = ''
              mkdir -p $out/share/applications
              substitute ${./host/vidya-demo.desktop} \
                $out/share/applications/vidya-demo.desktop \
                --replace-fail 'Exec=vidya-demo' "Exec=$out/bin/vidya-demo"
            '';
            meta = {
              description = "Vidya aesthetic showcase (cargo run host demo)";
              homepage = "https://tangled.org/nandi.uk/vidya";
              license = lib.licenses.mit;
              mainProgram = "vidya-demo";
            };
          };

          default = demo;
        }
      );

      apps = forAllSystems (
        system:
        let
          demo = self.packages.${system}.demo;
        in
        {
          demo = {
            type = "app";
            program = "${demo}/bin/vidya-demo";
          };
          default = self.apps.${system}.demo;
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
              # Prefer ~/.cargo/bin when it has cargo (rustup proxies); else active/default
              # rustup toolchain bin. Always keep ~/.cargo/bin for cargo-* helpers.
              export PATH="$HOME/.cargo/bin:$PATH"
              if ! command -v cargo >/dev/null 2>&1; then
                _vidya_tc=""
                if [ -n "''${RUSTUP_TOOLCHAIN:-}" ] && [ -x "$HOME/.rustup/toolchains/$RUSTUP_TOOLCHAIN/bin/cargo" ]; then
                  _vidya_tc="$HOME/.rustup/toolchains/$RUSTUP_TOOLCHAIN/bin"
                elif command -v rustup >/dev/null 2>&1; then
                  _vidya_which=$(rustup which cargo 2>/dev/null || true)
                  if [ -n "''${_vidya_which}" ] && [ -x "''${_vidya_which}" ]; then
                    _vidya_tc=$(dirname "''${_vidya_which}")
                  fi
                fi
                if [ -z "''${_vidya_tc}" ] && [ -f "$HOME/.rustup/settings.toml" ]; then
                  _vidya_def=$(sed -n 's/^default_toolchain = "\(.*\)"/\1/p' "$HOME/.rustup/settings.toml" | head -1)
                  if [ -n "''${_vidya_def}" ] && [ -x "$HOME/.rustup/toolchains/''${_vidya_def}/bin/cargo" ]; then
                    _vidya_tc="$HOME/.rustup/toolchains/''${_vidya_def}/bin"
                  fi
                fi
                if [ -z "''${_vidya_tc}" ]; then
                  for _vidya_c in \
                    "$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin" \
                    "$HOME/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin" \
                    "$HOME/.rustup/toolchains/default/bin" \
                    "$HOME/.rustup/toolchains/"*/bin; do
                    if [ -x "''${_vidya_c}/cargo" ]; then
                      _vidya_tc="''${_vidya_c}"
                      break
                    fi
                  done
                fi
                if [ -n "''${_vidya_tc}" ]; then
                  export PATH="''${_vidya_tc}:$PATH"
                fi
                unset _vidya_tc _vidya_def _vidya_which _vidya_c
              fi
              # rustup's gcc-ld can point at a GC'd nix store path; default to system cc+bfd.
              if [ -z "''${RUSTFLAGS:-}" ]; then
                export RUSTFLAGS="-C linker=cc -C link-arg=-fuse-ld=bfd"
              fi
              export ANDROID_NDK_HOME="''${ANDROID_NDK_HOME:-$HOME/.local/share/android-ndk-r29}"
              export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
              export ANDROID_HOME="''${ANDROID_HOME:-$HOME/.local/share/android-sdk}"
              export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$ANDROID_HOME/platform-tools:$PATH"
              export CC_x86_64_linux_android="''${CC_x86_64_linux_android:-x86_64-linux-android28-clang}"
              export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"
              export AR_x86_64_linux_android="''${AR_x86_64_linux_android:-llvm-ar}"
              # Wasm-capable Gleam for host/build.rs (examples/gleam_fib).
              if [ -z "''${GLEAM:-}" ]; then
                for candidate in \
                  "$HOME/code/gleam/target/debug/gleam" \
                  "$HOME/code/gleam/target/release/gleam" \
                  "$PWD/../gleam/target/debug/gleam"; do
                  if [ -x "$candidate" ]; then
                    export GLEAM="$candidate"
                    break
                  fi
                done
              fi
              echo "vidya — nix run | just gleam-app | just host | just fib | just gleam-gui | just gleam-shell | just waydroid | just shots''${GLEAM:+ (GLEAM=$GLEAM)}"
            '';
          };
        }
      );
    };
}
