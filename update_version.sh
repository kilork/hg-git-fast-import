#!/bin/bash

cargo update
cargo test

NEW=$1
CURRENT=`rg -m1 -N version Cargo.toml | cut -d\" -f2`

echo $CURRENT $NEW
rpl "$CURRENT" "$NEW" Cargo.toml README.md src/lib.rs snapcraft.yaml
git diff
read -p "Commit changes? " -n 1 -r
echo    # (optional) move to a new line
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    exit 1 || return 1 # handle exits from shell or function but don't exit interactive shell
fi
git commit -am"Release hg-git-fast-import $NEW"
git tag -s -e -m"Version $NEW" -m"Changes:" -m"- update dependencies" v$NEW
