# helix-discord-rpc

Discord RPC support for Helix via Steel.

<!--toc:start-->
- [helix-discord-rpc](#helix-discord-rpc)
  - [Requirements](#requirements)
  - [Installation](#installation)
  - [Features](#features)
  - [Acknowledgements](#acknowledgements)
<!--toc:end-->

## Requirements

- [Helix plugin fork](https://github.com/mattwparas/steel) 
- [Steel](https://github.com/mattwparas/steel)
- Alternatively, use the Nix Home Manager module via a flake

## Installation

<details>

<summary>Using forge</summary>

1. Install the plugin:

```sh
forge pkg install --git https://github.com/norinorin/helix-discord-rpc.git
```

2. `require` it in your `init.scm`:

```scm
(require (prefix-in helix-discord-rpc. "helix-discord-rpc/helix-discord-rpc.scm"))
```

</details>

<details>

<summary>Using Nix flake</summary>

Minimal setup:

```nix
# home.nix
{inputs, pkgs, ...}: let
  helix-discord-rpc = inputs.helix-discord-rpc.packages.${pkgs.stdenv.hostPlatform.system}.default;
in {
  programs.helix = {
    enable = true;
    package = pkgs.steelix; # unstable
  };

  xdg.configFile."helix/plugins/helix-discord-rpc".source =
    "${helix-discord-rpc}/share/helix-discord-rpc";

  xdg.configFile."helix/init.scm".text = ''
    (require "plugins/helix-discord-rpc/helix-discord-rpc.scm")
  '';

  home.file = {
    ".steel/native/libhelix_discord_rpc.so".source =
      "${helix-discord-rpc}/lib/libhelix_discord_rpc.so";
  };
}
  
```

</details>

To connect automatically, add the following line to your `init.scm`:

```
(discord-rpc-connect)
```

## Features

- [x] Activity update
  - [x] On workspace change
  - [x] On file change
- [x] IPC for multiple instances of Helix
- [x] Language icons
- [ ] Idle status
- [x] Cursor position
- [ ] Git status
- [ ] LSP workspace/file diagnostics
- [ ] Configuration (either in .scm or .toml)

## Acknowledgements

- [vscord](https://github.com/narcisbugeag/vscord): Assets (icons and languages mapping data)
  are sourced from the `vscord` project, licensed under the MIT license.
