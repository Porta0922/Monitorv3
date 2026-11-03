#!/bin/sh
set -eu
[ "$(id -u)" -eq 0 ] || { echo 'Run as root' >&2; exit 1; }
[ "$#" -eq 2 ] || { echo 'Usage: update-signed.sh BINARY DETACHED_SIGNATURE' >&2; exit 2; }
key=/etc/activity-monitor/release-public.pem
[ -f "$key" ] && [ ! -L "$key" ] || { echo 'Provision trusted release-public.pem first' >&2; exit 2; }
# Reject a trust anchor writable by non-root principals.
[ "$(stat -c %u "$key")" = 0 ] || exit 2
mode=$(stat -c %a "$key")
[ "$((0$mode & 0022))" -eq 0 ] || exit 2
stage=$(mktemp -d)
trap 'rm -f "$stage/candidate" "$stage/signature"; rmdir "$stage"' EXIT HUP INT TERM
cp -- "$1" "$stage/candidate"
cp -- "$2" "$stage/signature"
openssl dgst -sha256 -verify "$key" -signature "$stage/signature" "$stage/candidate"
hash=$(sha256sum "$stage/candidate" | cut -d ' ' -f 1)
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$script_dir/update-service.sh" "$stage/candidate" "$hash"
