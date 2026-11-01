{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
        libPath = with pkgs; lib.makeLibraryPath [
          libxkbcommon
          wayland
        ];
      in
    {
      packages.default = craneLib.buildPackage {
        CUSTOM_FONT_PATH = "${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSans.ttf";
        CUSTOM_TERMINAL = "alacritty";
        src = craneLib.cleanCargoSource ./.;
        buildInputs = [
        ];
        cargoExtraArgs = "--features wayland";
        nativeBuildInputs = [
          pkgs.makeWrapper
          pkgs.pkg-config
        ];
        postInstall = ''
            wrapProgram "$out/bin/pass-menu" --prefix LD_LIBRARY_PATH : "${libPath}"
            wrapProgram "$out/bin/ssh-menu" --prefix LD_LIBRARY_PATH : "${libPath}"
          '';
      };

      devShell = craneLib.devShell {
        packages = [
          pkgs.wayland
        ];
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        LD_LIBRARY_PATH = libPath;
        CUSTOM_FONT_PATH = "${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSans.ttf";
        CUSTOM_TERMINAL = "alacritty";
      };

      # devShell = with pkgs; mkShell {
      #   buildInputs = [
      #     cargo
      #     rust-analyzer
      #     rustPackages.clippy
      #     rustc
      #     rustfmt
      #     xorg.libxcb
      #   ];
      #   RUST_SRC_PATH = rustPlatform.rustLibSrc;
      #   LD_LIBRARY_PATH = libPath;
      # };
    });
}
