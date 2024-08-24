#!/bin/bash

[[ -z $TMPDIR ]] && TMPDIR=/tmp

if [[ -z $1 ]]; then
	for pid in $(ls $TMPDIR/zest.pid); do
		kill -HUP $pid
	done
else
	kill -HUP $1
fi
