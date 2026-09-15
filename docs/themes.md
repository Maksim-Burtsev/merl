# Themes

Every theme merl ships, on the sample project `merl --tutor` uses. `T` inside merl previews them
on your own code; `merl --theme NAME` or `theme = "NAME"` in `~/.config/merl/config.toml` picks
one.

## Dark

| Theme | |
|---|---|
| `tokyonight-moon`, default<br>[folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) | <img src="../assets/themes/tokyonight-moon.png" alt="merl in tokyonight-moon" width="480"> |
| `kanagawa-wave`<br>[rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim) | <img src="../assets/themes/kanagawa-wave.png" alt="merl in kanagawa-wave" width="480"> |
| `kanagawa-dragon`<br>[rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim) | <img src="../assets/themes/kanagawa-dragon.png" alt="merl in kanagawa-dragon" width="480"> |
| `rose-pine`<br>[rose-pine/neovim](https://github.com/rose-pine/neovim) | <img src="../assets/themes/rose-pine.png" alt="merl in rose-pine" width="480"> |
| `rose-pine-moon`<br>[rose-pine/neovim](https://github.com/rose-pine/neovim) | <img src="../assets/themes/rose-pine-moon.png" alt="merl in rose-pine-moon" width="480"> |
| `everforest-dark`<br>[sainnhe/everforest](https://github.com/sainnhe/everforest) | <img src="../assets/themes/everforest-dark.png" alt="merl in everforest-dark" width="480"> |
| `gruvbox-material-dark`<br>[sainnhe/gruvbox-material](https://github.com/sainnhe/gruvbox-material) | <img src="../assets/themes/gruvbox-material-dark.png" alt="merl in gruvbox-material-dark" width="480"> |
| `catppuccin-mocha`<br>[catppuccin/nvim](https://github.com/catppuccin/nvim) | <img src="../assets/themes/catppuccin-mocha.png" alt="merl in catppuccin-mocha" width="480"> |
| `flexoki-dark`<br>[kepano/flexoki-neovim](https://github.com/kepano/flexoki-neovim) | <img src="../assets/themes/flexoki-dark.png" alt="merl in flexoki-dark" width="480"> |
| `melange-dark`<br>[savq/melange-nvim](https://github.com/savq/melange-nvim) | <img src="../assets/themes/melange-dark.png" alt="merl in melange-dark" width="480"> |
| `nordfox`<br>[EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim) | <img src="../assets/themes/nordfox.png" alt="merl in nordfox" width="480"> |
| `shokunin-dark`<br>VS Code JSON in [`tools/themes-src/`](../tools/themes-src) | <img src="../assets/themes/shokunin-dark.png" alt="merl in shokunin-dark" width="480"> |

## Light

| Theme | |
|---|---|
| `rose-pine-dawn`<br>[rose-pine/neovim](https://github.com/rose-pine/neovim) | <img src="../assets/themes/rose-pine-dawn.png" alt="merl in rose-pine-dawn" width="480"> |
| `kanagawa-lotus`<br>[rebelot/kanagawa.nvim](https://github.com/rebelot/kanagawa.nvim) | <img src="../assets/themes/kanagawa-lotus.png" alt="merl in kanagawa-lotus" width="480"> |
| `everforest-light`<br>[sainnhe/everforest](https://github.com/sainnhe/everforest) | <img src="../assets/themes/everforest-light.png" alt="merl in everforest-light" width="480"> |
| `flexoki-light`<br>[kepano/flexoki-neovim](https://github.com/kepano/flexoki-neovim) | <img src="../assets/themes/flexoki-light.png" alt="merl in flexoki-light" width="480"> |
| `catppuccin-latte`<br>[catppuccin/nvim](https://github.com/catppuccin/nvim) | <img src="../assets/themes/catppuccin-latte.png" alt="merl in catppuccin-latte" width="480"> |
| `gruvbox-material-light`<br>[sainnhe/gruvbox-material](https://github.com/sainnhe/gruvbox-material) | <img src="../assets/themes/gruvbox-material-light.png" alt="merl in gruvbox-material-light" width="480"> |
| `melange-light`<br>[savq/melange-nvim](https://github.com/savq/melange-nvim) | <img src="../assets/themes/melange-light.png" alt="merl in melange-light" width="480"> |
| `dawnfox`<br>[EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim) | <img src="../assets/themes/dawnfox.png" alt="merl in dawnfox" width="480"> |
| `dayfox`<br>[EdenEast/nightfox.nvim](https://github.com/EdenEast/nightfox.nvim) | <img src="../assets/themes/dayfox.png" alt="merl in dayfox" width="480"> |
| `tokyonight-day`<br>[folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) | <img src="../assets/themes/tokyonight-day.png" alt="merl in tokyonight-day" width="480"> |
| `bluloco-light`<br>[uloco/bluloco.nvim](https://github.com/uloco/bluloco.nvim) | <img src="../assets/themes/bluloco-light.png" alt="merl in bluloco-light" width="480"> |
| `shokunin-light`<br>VS Code JSON in [`tools/themes-src/`](../tools/themes-src) | <img src="../assets/themes/shokunin-light.png" alt="merl in shokunin-light" width="480"> |

## Adding a theme

Themes are ported from their Neovim originals, never from a VS Code port, one command each:

```sh
tools/port-theme.sh https://github.com/sainnhe/everforest everforest everforest-light \
  "set background=light" "let g:everforest_background='medium'"
```

It clones the repo, applies the colorscheme in headless Neovim, maps the resolved highlight groups
to TextMate scopes with `tools/nvim2tmtheme.py` (one table for every theme), copies the licence to
`themes/` and prints the rows to add to `THEMES` in `src/theme.rs` and to this page. Only whoever
ports needs Neovim; the build never touches it. The two Shokunin themes are generated from their
VS Code JSON sources with `python3 tools/vscode2tmtheme.py in.json out.tmTheme "Name"`.

The screenshot comes from `cargo build --release && tools/theme-shots.sh NAME`. A test fails while
a theme in `THEMES` has no row or no screenshot here.

## Licences

The ported themes keep their authors' licences, shipped next to them in `themes/`:

- MIT: [catppuccin](../themes/LICENSE-catppuccin), [everforest](../themes/LICENSE-everforest),
  [flexoki](../themes/LICENSE-flexoki), [gruvbox-material](../themes/LICENSE-gruvbox-material),
  [kanagawa](../themes/LICENSE-kanagawa), [melange](../themes/LICENSE-melange),
  [nightfox](../themes/LICENSE-nightfox), [rose-pine](../themes/LICENSE-rose-pine)
- Apache-2.0: [tokyonight](../themes/LICENSE-tokyonight)
- LGPL-3.0: [bluloco](../themes/LICENSE-bluloco). A converted palette is data, not linked code, so
  the LGPL does not reach the binary.
