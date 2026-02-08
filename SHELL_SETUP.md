# Shell Setup Guide

Termina runs each command through a shell. By default it uses your system shell (`$SHELL`), but you can customise the shell binary and an **init script** that runs before every command.

## Shell Path vs Init Script

- **Shell path** — The shell binary used to execute commands (e.g. `/bin/zsh`, `/bin/bash`). Leave blank to use `$SHELL`.
- **Init script** — A snippet that runs at the start of every command session. Use it to set up `PATH`, source configs, or initialise version managers. This is necessary because GUI apps on macOS don't inherit your terminal's environment.

## Common Init Scripts

### nvm (Node Version Manager)

```sh
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
```

### fnm (Fast Node Manager)

```sh
eval "$(fnm env)"
```

### Homebrew PATH

```sh
eval "$(/opt/homebrew/bin/brew shellenv)"
```

### pyenv

```sh
eval "$(pyenv init -)"
```

### rbenv

```sh
eval "$(rbenv init -)"
```

### Source ~/.zshrc

```sh
unset npm_config_prefix 2>/dev/null
[ -f "$HOME/.zshrc" ] && . "$HOME/.zshrc" 2>/dev/null
```

> **Note:** The `unset npm_config_prefix` line prevents a common conflict where Homebrew's node sets this variable, which breaks nvm. Always include it if you use both Homebrew and nvm.

### Source ~/.bashrc

```sh
[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc" 2>/dev/null
```

## Combo Examples

### Homebrew + nvm

```sh
unset npm_config_prefix 2>/dev/null
eval "$(/opt/homebrew/bin/brew shellenv)"
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"
```

### Homebrew + pyenv

```sh
eval "$(/opt/homebrew/bin/brew shellenv)"
eval "$(pyenv init -)"
```

### Full zshrc (catches everything)

```sh
unset npm_config_prefix 2>/dev/null
[ -f "$HOME/.zshrc" ] && . "$HOME/.zshrc" 2>/dev/null
```

This sources your entire zsh config, which will pick up any version managers, PATH additions, and aliases you've configured. It's the simplest option but may be slower if your `.zshrc` is large.

## Tips

- Keep init scripts short — they run before every command launch.
- If a command can't find a binary (e.g. `node`, `python`), it's usually a missing PATH entry. Add the relevant init script.
- Use `which <binary>` in your terminal to find where a tool lives, then make sure that path is on `PATH` in the init script.
- You can test your init script by running `sh -c '<your init script> && echo $PATH'` in a terminal.
