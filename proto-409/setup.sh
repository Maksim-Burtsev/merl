#!/bin/sh
# Rebuilds /tmp/409: a shop repo with agent branches, clean shell rcs, fish and bash 5 from Homebrew bottles.
# Then: tools/cast.py --bin /tmp/409/run-zsh-a --keys proto-409/zsh.steps -o a-zsh.gif --size 100x16 --type-delay 0.07 --key-delay 0.3 --project /tmp/409/home/shop
set -e
here=$(cd "$(dirname "$0")" && pwd)
rm -rf /tmp/409 && mkdir -p /tmp/409/home /tmp/409/bin /tmp/409/shells /tmp/409/site-a /tmp/409/site-b /tmp/409/zdot-a /tmp/409/zdot-b /tmp/409/zdot-now
cp "$here/../target/release/merl" /tmp/409/bin/merl
cp "$here/shell.sh" "$here/bashrc" /tmp/409/
export HOME=/tmp/409/home GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com
git init -q --bare -b main /tmp/409/origin.git
git clone -q /tmp/409/origin.git /tmp/409/home/shop 2>/dev/null
cd /tmp/409/home/shop
mkdir app && printf 'def checkout(cart):\n    return sum(i.price for i in cart)\n' > app/cart.py && printf '# shop\n' > README.md
git add -A && git commit -qm init && git push -q origin main
for b in claude/task-2ars83 claude/task-9kq1vd claude/fix-cart-total agent/fix-login-rate agent/refund feature/checkout-v2 develop; do git branch $b; git push -q origin $b; done
for b in claude/task-7hx2pm codex/add-coupons release/2.4 hotfix/tax-rounding; do git branch $b; git push -q origin $b; git branch -qD $b; done
git remote set-head origin main
cd /tmp/409
for d in a b now; do printf 'fpath=(/tmp/409/site-%s $fpath)\nautoload -U compinit && compinit -u -d /tmp/409/zdot-%s/.zcompdump\nPROMPT=%s\n' $d $d "'%~ %# '" > zdot-$d/.zshrc; done
(cd bin && PATH=/tmp/409/bin:$PATH COMPLETE=zsh merl > /tmp/409/site-a/_merl)
MERL_STATIC_COMPLETION=zsh bin/merl > site-b/_merl
HOMEBREW_NO_AUTO_UPDATE=1 brew fetch -q fish bash readline
for f in fish bash readline; do tar -xzf "$(brew --cache $f)" -C shells; done
for k in zsh-a zsh-b zsh-now bash fish; do printf '#!/bin/sh\nexec /tmp/409/shell.sh %s\n' $k > run-$k; chmod +x run-$k; done
