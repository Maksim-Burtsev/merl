#!/bin/sh
# One shell per kind, in /tmp/409r3/home/shop, with merl's dynamic completion loaded.
R=/tmp/409r3
S=${SHELLS:-$R/shells}
kind=$1
export PATH=$R/bin:/usr/bin:/bin TERM=xterm-256color LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8
cd $R/home/shop
case $kind in
  zsh-clean) HOME=$R/home ZDOTDIR=$R/zdot-clean exec zsh -d -i ;;
  zsh-omz) HOME=$R/home ZDOTDIR=$R/zdot-omz exec zsh -i ;;
  zsh-real) exec env -i HOME=/Users/mburtsev TERM=xterm-256color LANG=en_US.UTF-8 PATH=/usr/bin:/bin zsh -i ;;
  bash) HOME=$R/home exec $S/bash/5.3.20/bin/bash --noprofile --rcfile $R/bashrc -i ;;
  bash3) HOME=$R/home exec /bin/bash --noprofile --rcfile $R/bashrc -i ;;
  fish) HOME=$R/home exec $S/fish/4.9.3/bin/fish --no-config --private -C 'COMPLETE=fish merl | source; function fish_prompt; printf "%s \$ " (prompt_pwd); end; function fish_greeting; end' ;;
esac
