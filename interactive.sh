#!/bin/sh
INITIAL_QUERY=""
RG_PREFIX="marks --no-markdown --path ~/ --query "
FZF_DEFAULT_COMMAND="$RG_PREFIX '$INITIAL_QUERY'" \
  fzf --bind "change:reload:$RG_PREFIX {q} || true" \
      --ansi --phony --query "$INITIAL_QUERY" \
      --layout=reverse \
      --preview 'CURRLINE=$(echo {} | cut -d: -f2); bat --style=numbers --color=always --highlight-line $CURRLINE --line-range $CURRLINE: `echo {} | cut -d: -f1`'
