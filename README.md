# helix-discord-rpc

Discord RPC support for Helix via Steel.

<!--toc:start-->
- [helix-discord-rpc](#helix-discord-rpc)
  - [Requirements](#requirements)
  - [Installation](#installation)
  - [Examples](#examples)
  - [Features](#features)
  - [Acknowledgements](#acknowledgements)
<!--toc:end-->

## Requirements

- [Helix plugin fork](https://github.com/mattwparas/helix) 
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

## Examples

<details>

<summary>Connecting automatically and reset elapsed time on file change</summary>

```scm
; init.scm
(require "plugins/helix-discord-rpc/helix-discord-rpc.scm")

;; Options: "state" | "file" | "workspace" | "never"
;; Default: "state" -> elapsed time resets on any state change (cursor movement, line change, etc.)
(discord-rpc-set-reset-mode "file") ;; elapsed time resets on file change

(discord-rpc-connect)
```

</details>

<details>

<summary>Hidden workspace and filename</summary>

```scm
; init.scm
(require-builtin steel/strings)

(require "plugins/helix-discord-rpc/helix-discord-rpc.scm")
(require "plugins/helix-discord-rpc/utils.scm")

(discord-rpc-register-details-fn
  (lambda ()
    "In a workspace"))

; naive impl
(define (a-or-an word)
  (let ([w (string-downcase word)])
    (if (or (starts-with? w "a")
         (starts-with? w "e")
         (starts-with? w "i")
         (starts-with? w "o")
         (starts-with? w "u"))
      "an"
      "a")))

(discord-rpc-register-state-fn
  (lambda ()
    (let ([lang (discord-rpc-current-language)])
      (if (string=? lang "")
        "Editing a file"
        (string-append
          "Editing "
          (a-or-an lang)
          " "
          lang
          " file")))))

(discord-rpc-connect)
```

</details>

<details>

<summary>Black/whitelisting</summary>

```scm
; init.scm
(require "plugins/helix-discord-rpc/helix-discord-rpc.scm")
(require "plugins/helix-discord-rpc/utils.scm")

; Each `discord-rpc-register-should-display-fn` call overrides
; the previous one, so only use this function once!
; To apply multiple rules, compose them in a single lambda.

; 1. Exact-match blacklist
(define ignored-workspaces '("secret-project" "confidential"))

(discord-rpc-register-should-display-fn
  (lambda ()
    (let ([folder (discord-rpc-basename (discord-rpc-current-workspace-path))])
      (not (member folder ignored-workspaces)))))

; 2. Substring-match blacklist
(discord-rpc-register-should-display-fn
  (lambda ()
    (let ([path (discord-rpc-current-workspace-path)])
      (not (string-contains? path "/private/")))))

; 3. Language whitelist
(discord-rpc-register-should-display-fn
  (lambda ()
    (let ([lang (discord-rpc-current-language)])
      (or (string=? lang "rust")
          (string=? lang "scheme")))))

(discord-rpc-connect)
```

</details>

## Features

- [x] Activity update
  - [x] On workspace change
  - [x] On file change
- [x] IPC for multiple instances of Helix
- [x] Language icons
- [ ] Idle status (to be implemented in lib)
- [x] Cursor position
- [ ] Git status (to be implemented in steel?)
- [ ] LSP workspace/file diagnostics (to be implemented in steel)
- [x] Configuration

## Acknowledgements

- [vscord](https://github.com/narcisbugeag/vscord): Assets (icons and languages mapping data)
  are sourced from the `vscord` project, licensed under the MIT license.
