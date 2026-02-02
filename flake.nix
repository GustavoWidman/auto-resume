{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    devshell.url = "github:numtide/devshell";
  };

  outputs =
    inputs@{
      flake-parts,
      systems,
      devshell,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import systems;
      imports = [
        devshell.flakeModule
        (import ./nix/package.nix inputs)
      ];
      perSystem =
        { pkgs, ... }:
        {
          devshells.default = {
            env = [
              {
                name = "PKG_CONFIG_PATH";
                value = pkgs.lib.makeSearchPath "lib/pkgconfig" (
                  with pkgs;
                  [
                    graphite2.dev
                    freetype.dev
                    libpng.dev
                    icu.dev
                    harfbuzz.dev
                    zlib.dev
                    glib.dev
                    cairo.dev
                  ]
                );
              }
              {
                name = "FONTCONFIG_FILE";
                value = pkgs.makeFontsConf { fontDirectories = [ pkgs.tex-gyre ]; };
              }
            ];
            devshell = {
              packages = with pkgs; [
                graphite2.dev
                freetype.dev
                libpng.dev
                icu.dev
                harfbuzz.dev
                zlib.dev
                pkg-config
                glib.dev
                cairo.dev
                tex-gyre
                fontconfig
              ];
              startup = {
                vars.text = ''
                  export CPATH="${
                    pkgs.lib.makeSearchPath "include" (
                      with pkgs;
                      [
                        graphite2.dev
                        freetype.dev
                        libpng.dev
                        icu.dev
                        harfbuzz.dev
                        zlib.dev
                        glib.dev
                        cairo.dev
                      ]
                    )
                  }:$CPATH"
                  export LIBRARY_PATH="${
                    pkgs.lib.makeLibraryPath (
                      with pkgs;
                      [
                        graphite2
                        freetype
                        libpng
                        icu
                        harfbuzz
                        zlib
                        glib
                        cairo
                      ]
                    )
                  }:$LIBRARY_PATH"
                '';
              };
              motd = "";
            };
          };
        };
    };
}
