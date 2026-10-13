#!/bin/sh
# Rebuilds /tmp/409r3: an origin with ~260 branches (team task branches, agent branches, odd names),
# a clone that fetched all but three of them, a merl wrapper that prints its argv, clean shell rcs.
set -e
here=$(cd "$(dirname "$0")" && pwd)
R=${R:-/tmp/409r3}
rm -rf $R && mkdir -p $R/home $R/bin $R/site $R/zdot-clean $R/zdot-omz $R/cache
cp "$here/../../target/release/merl" $R/bin/merl-real
mkdir -p $R/realbin && cp $R/bin/merl-real $R/realbin/merl
cat > $R/bin/merl <<'W'
#!/bin/sh
[ -n "$COMPLETE" ] && exec /tmp/409r3/bin/merl-real "$@"
out='merl got:'; for a in "$@"; do out="$out [$a]"; done; printf '%s END\n' "$out"
W
chmod +x $R/bin/merl
export HOME=$R/home GIT_AUTHOR_NAME=dev GIT_AUTHOR_EMAIL=dev@example.com GIT_COMMITTER_NAME=dev GIT_COMMITTER_EMAIL=dev@example.com
git init -q --bare -b main $R/origin.git
git clone -q $R/origin.git $R/home/shop 2>/dev/null
cd $R/home/shop
mkdir app && printf 'def login(user, password):\n    return user.check(password)\n' > app/auth.py
printf '# shop\n' > README.md && git add -A && git commit -qm init && git push -q origin main
python3 "$here/branches.py" > $R/branches.txt
while IFS= read -r b; do git update-ref "refs/heads/$b" HEAD; done < $R/branches.txt
git switch -q dev-harbor/2505-digest-login-mobile
printf 'def login(user, password, digest=False):\n    if digest:\n        return user.check_digest(password)\n    return user.check(password)\n' > app/auth.py
git commit -qam 'Digest login on mobile'
git switch -q 'фича/поиск-заказов'
printf 'def find_orders(query):\n    return [o for o in orders() if query in o.title]\n' > app/orders.py
git add app/orders.py && git commit -qm 'Order search'
git switch -q main
git push -q origin 'refs/heads/*:refs/heads/*'
git for-each-ref --format='%(refname)' refs/heads | grep -v '^refs/heads/main$' | while IFS= read -r r; do git update-ref -d "$r"; done
git fetch -q origin
git remote set-head origin main
for b in dev-harbor/1234-fix-login claude/task-2ars83 agent/refund; do git branch -q "$b" "origin/$b"; done
git clone -q $R/origin.git $R/pusher 2>/dev/null
cd $R/pusher
for b in dev-quartz/7777-pushed-after-fetch claude/task-late01 dev-falcon/8123-new-onboarding; do
  git switch -q -c "$b" main
  printf 'def login(user, password):\n    if user.locked:\n        return False\n    return user.check(password)\n' > app/auth.py
  git commit -qam "Lock out ($b)"
  git push -q origin "$b"
done
cd $R
printf 'autoload -U compinit && compinit -u -d %s/zdot-clean/.zcompdump\nsource <(COMPLETE=zsh merl)\nPROMPT=%s\n' $R "'%~ %# '" > zdot-clean/.zshrc
cat > zdot-omz/.zshrc <<Z
export ZSH=$(cd ~mburtsev 2>/dev/null && pwd)/.oh-my-zsh ZSH_CACHE_DIR=$R/cache ZSH_COMPDUMP=$R/zdot-omz/.zcompdump
DISABLE_AUTO_UPDATE=true
zstyle ':omz:update' mode disabled
ZSH_THEME=robbyrussell
plugins=(git virtualenv)
source \$ZSH/oh-my-zsh.sh
source <(COMPLETE=zsh merl)
Z
mkdir -p $R/zdot-now && grep -v COMPLETE zdot-omz/.zshrc > zdot-now/.zshrc
printf 'source <(COMPLETE=bash merl)\nPS1=%s\n' "'\\w \\$ '" > bashrc
for k in omz clean now; do
  printf '#!/bin/sh\nexport PATH=%s/realbin:/opt/homebrew/bin:/usr/bin:/bin TERM=xterm-256color LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 HOME=%s/home ZDOTDIR=%s/zdot-%s\ncd %s/home/shop\nexec zsh -d -i\n' $R $R $R $k $R > $R/run-$k
  chmod +x $R/run-$k
done
