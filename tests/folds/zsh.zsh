#!/usr/bin/env zsh
# fi done esac in a comment

() {  # f: 4-8
  local -a parts
  parts=(${(s:/:)PWD})  # f: 4-8
  print -l $parts
}

function greet {  # f: 10-19
  local name=${1:-world}
  if [[ -n $name ]]; then  # f: 12-18
    print "hello $name"
  elif [[ $name == fi ]]; then
    print "fi"  # f: 10-19
  else
    print 'done'
  fi
}

prompt_setup() {  # f: 21-43
  {
    for f in ~/.zsh/*.zsh; do  # f: 23-25
      source $f
    done
  } always {  # f: 21-43
    unset f  # f: 21-43
  }
  case $TERM in  # f: 29-36
    xterm*)  # f: 21-43
      print xterm
      ;;
    *)
      print other
      ;;
  esac
  while read -r line; do  # f: 37-39
    print -- $line
  done < $HISTFILE
  cat <<EOF  # f: 40-42
  esac fi } done
EOF
}
