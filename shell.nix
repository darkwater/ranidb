with import <nixpkgs> {
  overlays = [
    (import (builtins.fetchGit {
      url = "https://git.dark.red/darkwater/onyx";
      ref = "master";
    }) {}).overlay
  ];
};

onyx-shells.rust {
  name = "ranidb";
  nightly = "2020-09";
}
