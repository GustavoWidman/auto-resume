{
  crane,
  fenix,
  ...
}:
{
  perSystem =
    {
      pkgs,
      lib,
      system,
      ...
    }:
    let
      toolchain = fenix.packages.${system}.fromToolchainFile {
        file = ../rust-toolchain.toml;
        sha256 = "sha256-WInuoEKQQy+RPFLGMp0sLDxCzFKfiixAE6s1ssLYxcc=";
      };
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
      root = ../.;

      args = {
        src = lib.fileset.toSource {
          inherit root;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources root)
            # (lib.fileset.fileFilter (file: file.hasExt "md") root)
          ];
        };
        strictDeps = true;

        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [ openssl ];
      };

      bin = craneLib.buildPackage (
        args
        // {
          cargoArtifacts = craneLib.buildDepsOnly args;
        }
      );
    in
    {
      checks.auto-resume = bin;

      packages.default = bin;

      devshells.default = {
        packages = [
          toolchain
        ];
        commands = [
          {
            help = "";
            name = "hot";
            command = "${pkgs.watchexec}/bin/watchexec -e rs -w src -w Cargo.toml -w Cargo.lock -r ${toolchain}/bin/cargo run -- $@";
          }
        ];
      };
    };
}
