{
  inputs = {
    nixpkgs.follows = "fenix/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
      fenix-pkgs = fenix.packages."${system}";
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
          (fenix-pkgs.complete.withComponents [
                       "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
          ])
          fenix-pkgs.rust-analyzer
          pkgs.bacon
          pkgs.evcxr
        ] ++ runLibs;

        # make the dynamic linker see the libs at runtime
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runLibs;
      };
    });
}

