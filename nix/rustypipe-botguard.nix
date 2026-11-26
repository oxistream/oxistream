{
  lib,
  stdenv,
  rustPlatform,
  fetchCrate,
  fetchurl,
}:

let
  version = "0.1.2";
  # Must match the `v8` version in the crate's Cargo.lock.
  v8Version = "130.0.7";
  v8Hashes = {
    aarch64-apple-darwin = "sha256-9tvQD08OdW6GoNnx/3vgS27D9Aj9YdQdNJ9SgNvwAOo=";
    x86_64-apple-darwin = "sha256-QwcxsJHf+n3o7f8PyXRGqdwdNAr5mgHrQEw0VTgeEsA=";
    x86_64-unknown-linux-gnu = "sha256-pkdsuU6bAkcIHEZUJOt5PXdzK424CEgTLXjLtQ80t10=";
    aarch64-unknown-linux-gnu = "sha256-vu/ns1q+53FZ98tVCWZmsHYwwRBH1fOnTdBlhjcpkVo=";
  };
  target = stdenv.hostPlatform.rust.rustcTarget;
  librusty_v8 = fetchurl {
    url = "https://github.com/denoland/rusty_v8/releases/download/v${v8Version}/librusty_v8_release_${target}.a.gz";
    hash = v8Hashes.${target} or (throw "rustypipe-botguard: unsupported target ${target}");
  };
in
rustPlatform.buildRustPackage {
  pname = "rustypipe-botguard";
  inherit version;

  src = fetchCrate {
    pname = "rustypipe-botguard";
    inherit version;
    hash = "sha256-l9prqMEg9yoz8uMquT15a8i+5Cao9w95sjywKitR9JI=";
  };

  cargoHash = "sha256-hPKjdP4X/iNNUlD8sjv7pKeRoDD8+Pj2xp1h8XOd/bs=";

  # The v8 build script would otherwise download this archive.
  RUSTY_V8_ARCHIVE = librusty_v8;

  doCheck = false;

  meta = {
    description = "Run YouTube Botguard challenges and generate PO tokens";
    homepage = "https://codeberg.org/ThetaDev/rustypipe-botguard";
    license = lib.licenses.mit;
    mainProgram = "rustypipe-botguard";
  };
}
