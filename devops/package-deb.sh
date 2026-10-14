#!/usr/bin/env bash
set -euo pipefail

dynamic_binary=$(realpath "${1:?dynamic binary}")
static_binary=$(realpath "${2:?static binary}")
release_version=${3:?release version}
if [[ ! "$release_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.+-]+)?$ ]]; then
    echo 'Expected a Cargo-style release version' >&2
    exit 1
fi
repository=$(cd "$(dirname "$0")/.." && pwd)
package_workspace=$(mktemp -d)
trap 'rm -rf "$package_workspace"' EXIT

# A static-labelled package must not silently ship an ELF interpreter.
program_headers=$(readelf -l "$static_binary")
if grep -q INTERP <<< "$program_headers"; then
    echo 'The static artifact still requires a dynamic loader' >&2
    exit 1
fi

mkdir -p "$package_workspace/debian"
cat > "$package_workspace/debian/control" <<'EOF'
Source: pg-actions
Section: utils
Priority: optional
Maintainer: Cristian Iordachescu <53430981+CristiCriss17G@users.noreply.github.com>

Package: pg-actions
Architecture: any
Depends: ${shlibs:Depends}
Description: PostgreSQL administrative CLI
EOF
install -Dm755 "$dynamic_binary" "$package_workspace/debian/pg-actions/usr/bin/pg_actions"
dependencies=$(cd "$package_workspace" && dpkg-shlibdeps -O -edebian/pg-actions/usr/bin/pg_actions)
dependencies=${dependencies#shlibs:Depends=}

for kind in dynamic static; do
    package_root="$package_workspace/$kind"
    mkdir -p "$package_root/DEBIAN"
    cp "$repository/debian/$kind/DEBIAN/control" "$package_root/DEBIAN/control"
    sed -i "s/^Version:.*/Version: ${release_version}-1/" "$package_root/DEBIAN/control"
    if [ "$kind" = dynamic ]; then
        sed -i "s/\${shlibs:Depends}/$dependencies/" "$package_root/DEBIAN/control"
        selected_binary=$dynamic_binary
        suffix=''
    else
        selected_binary=$static_binary
        suffix='-static'
    fi
    install -Dm755 "$selected_binary" "$package_root/usr/bin/pg_actions"
    install -Dm644 "$repository/LICENSE" "$package_root/usr/share/doc/pg-actions/LICENSE"
    install -Dm644 "$repository/debian/$kind/usr/share/doc/pg-actions/copyright" "$package_root/usr/share/doc/pg-actions/copyright"
    dpkg-deb --root-owner-group --build "$package_root" "$repository/pg_actions${suffix}_${release_version}_amd64.deb"
done
