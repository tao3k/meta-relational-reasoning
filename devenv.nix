{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  gerbilCommand = ''
    exec env -u CC -u CFLAGS -u CPPFLAGS -u LDFLAGS \
      -u CPATH -u C_INCLUDE_PATH -u CPLUS_INCLUDE_PATH -u LIBRARY_PATH \
      -u NIX_CFLAGS_COMPILE -u NIX_LDFLAGS -u DEVELOPER_DIR -u SDKROOT \
      gerbil "$@"
  '';
in
{
  dotenv.enable = true;
  dotenv.filename = [ ".env" ];

  # https://devenv.sh/basics/
  env.GREET = "devenv";
  # Request all local performance cores; ASP Gerbil still applies its bounded
  # memory-admission ceiling before launching compiler workers.
  env.GERBIL_BUILD_CORES = "12";
  # https://devenv.sh/packages/
  packages = [
    pkgs.pkg-config
    pkgs.openssl
    pkgs.protobuf
    pkgs.just
    pkgs.fd
    pkgs.ripgrep
    pkgs.mermaid-cli
    pkgs.tlaplus
    pkgs.eza
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    # Ensure rust can link python library
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
    ];
  };

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';
  # Homebrew Gerbil/Gambit must use the host SDK.  Keep Nix's compiler
  # environment for Rust, and quarantine it only at the standard Gerbil edge.
  scripts.mrr-gerbil.exec = gerbilCommand;
  scripts.mrr-gerbil-deps.exec = "mrr-gerbil deps --install";
  # Preserve devenv's Rust compiler/SDK while inheriting gxpkg's canonical
  # project package path. Cargo's AOT adapter sanitizes its Gerbil children.
  scripts.mrr-cargo.exec = ''
    exec gerbil env cargo "$@"
  '';

  # https://devenv.sh/basics/
  enterShell = "";

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };
  # https://devenv.sh/tests/
  enterTest = "";

  git-hooks.hooks = {
    shellcheck.enable = true;
    nixfmt.enable = true;
    clippy.enable = true;
    ruff.enable = true;
    clippy.packageOverrides.cargo = config.languages.rust.toolchain.cargo;
    clippy.packageOverrides.clippy = config.languages.rust.toolchainPackage;
    clippy.settings.allFeatures = true;
  };
  # See full reference at https://devenv.sh/reference/options/
}
