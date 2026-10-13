#!/bin/sh
kind=$1
export HOME=/tmp/409/home PATH=/tmp/409/bin:/usr/bin:/bin TERM=xterm-256color LANG=en_US.UTF-8
cd /tmp/409/home/shop
case $kind in
  zsh-a|zsh-b|zsh-now) ZDOTDIR=/tmp/409/zdot-${kind#zsh-} exec zsh -d -i ;;
  bash) DYLD_LIBRARY_PATH=$(echo /tmp/409/shells/readline/*/lib):/opt/homebrew/opt/ncurses/lib:/opt/homebrew/opt/gettext/lib exec $(echo /tmp/409/shells/bash/*/bin/bash) --noprofile --rcfile /tmp/409/bashrc -i ;;
  bash3) exec /bin/bash --noprofile --rcfile /tmp/409/bashrc -i ;;
  fish) DYLD_LIBRARY_PATH=/opt/homebrew/opt/pcre2/lib exec $(echo /tmp/409/shells/fish/*/bin/fish) --no-config --private -C 'COMPLETE=fish merl | source; function fish_prompt; printf "%s \$ " (prompt_pwd); end; function fish_greeting; end' ;;
esac
