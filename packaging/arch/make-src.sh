#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
repo_root="$(git -C ../.. rev-parse --show-toplevel)"

pkgname=$(grep -E '^pkgname=' PKGBUILD | cut -d= -f2-)
pkgver=$(grep -E '^pkgver=' PKGBUILD | cut -d= -f2-)

archive="$pkgname-$pkgver.tar.gz"

git -C "$repo_root" archive --format=tar.gz --prefix="$pkgname-$pkgver/" \
  -o "$(pwd)/$archive" "v$pkgver"

sum=$(sha256sum "$archive" | cut -d' ' -f1)

sed -i "s/^sha256sums=('.*')/sha256sums=('$sum')/" PKGBUILD

echo "Generated $archive (sha256: $sum)"