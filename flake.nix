{
  description = "rust-overlay devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };
        lib = pkgs.lib;

        # Rust toolchain
        customRustToolchain = pkgs.rust-bin.beta.latest.default;
        craneLib =
          (crane.mkLib pkgs).overrideToolchain customRustToolchain;
        projectName =
          (craneLib.crateNameFromCargoToml {
            cargoToml = ./Cargo.toml;
          }).pname;
        projectVersion =
          (craneLib.crateNameFromCargoToml {
            cargoToml = ./Cargo.toml;
          }).version;

        pythonVersion = pkgs.python312;
        wheelTail = "cp313-cp313-linux_x86_64"; # Change if pythonVersion changes
        wheelName = "${projectName}-${projectVersion}-${wheelTail}.whl";
        crateCfg = {
          # src = craneLib.cleanCargoSource (craneLib.path ./.);
          src = ./.;
          nativeBuildInputs = [pythonVersion];
        };

        crateWheel = (craneLib.buildPackage (crateCfg
          // {
            pname = projectName;
            version = projectVersion;
            # cargoArtifacts = crateArtifacts;
          })).overrideAttrs (old: {
          nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.maturin];
          buildPhase =
            old.buildPhase
            + ''
              maturin build --offline --target-dir ./target
            '';
          installPhase =
            old.installPhase
            + ''
              ls target/wheels/
              cp target/wheels/${wheelName} $out/
            '';
        });
      in rec {
        # Package outputs
        packages = rec {
          default = crateWheel;
          pythonEnv =
            pythonVersion.withPackages
            (ps: [(lib.pythonPackage ps)] ++ (with ps; [ipython numpy pillow]));
        };

        lib = {
          # To use in other builds with the "withPackages" call
          pythonPackage = ps:
            ps.buildPythonPackage rec {
              pname = projectName;
              format = "wheel";
              version = projectVersion;
              src = "${crateWheel}/${wheelName}";
              doCheck = false;
              pythonImportsCheck = [projectName];
            };
        };

        devShells = rec {
          rust = pkgs.mkShell {
            name = "rust-devshell";
            nativeBuildInputs = with pkgs; [
              pkg-config
              rust-analyzer
              maturin
              wayland
              wayland-protocols
            ];

            buildInputs = with pkgs; [
              (customRustToolchain.override
                {
                  extensions = ["rust-src"];
                })
              eza
              fd
              jq
              # Python with dependencies
              (python3.withPackages (ps:
                with ps; [
                  numpy
                  pillow
                ]))
            ];

            shellHook = ''
              alias ls=eza
              alias find=fd
              export PYTHONPATH="$PWD/python:$PYTHONPATH"
            '';
          };

          python = pkgs.mkShell {
            name = "python-devshell";
            nativeBuildInputs = [packages.pythonEnv];

            buildInputs = with pkgs; [
              pythonVersion
              (pythonVersion.withPackages (ps: [
                pythonVersion.pkgs.ipython
                pythonVersion.pkgs.numpy
                pythonVersion.pkgs.pillow
              ]))
            ];
            shellHook = ''
              export PYTHONPATH="$PWD/python:$PYTHONPATH"
            '';
          };

          default = rust;
        };
        apps = rec {
          ipython = {
            type = "app";
            program = "${packages.pythonEnv}/bin/ipython";
          };
          default = ipython;
        };
      }
    );
}
