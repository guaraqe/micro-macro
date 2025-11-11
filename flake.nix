{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
      # libs for Wayland + X11 + GL + Vulcan loader (covers glow/wgpu paths)
      runLibs = with pkgs; [
        wayland
        libxkbcommon
        xorg.libX11
        xorg.libXcursor
        xorg.libXi
        xorg.libXrandr
        libGL
        vulkan-loader
      ];
    in {
      devShells.default = pkgs.mkShell {
        packages = [
          pkgs.rustup
          pkgs.rust-analyzer
          pkgs.cargo-watch
          pkgs.bacon
        ] ++ runLibs;

        # make the dynamic linker see the libs at runtime
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runLibs;
      };
    });
}

