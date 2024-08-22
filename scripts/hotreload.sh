#!/bin/bash

[[ -z $TMPDIR ]] && TMPDIR=/tmp

for pid in $(ls $TMPDIR/zest.pid); do
	kill -HUP $pid
done
