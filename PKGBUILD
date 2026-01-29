pkgname=bwsandbox
pkgver=0.4.0
pkgrel=1
pkgdesc="Sandbox utility to orchestrate bwrap and other services configuration"
arch=('x86_64')
url="https://github.com/a-liashenko/bwsandbox"
license=('GPL-3.0-or-later')
depends=('bubblewrap' 'libseccomp')
optdepends=('xdg-dbus-proxy: D-Bus filtering support'
            'slirp4netns: Network isolation support')
makedepends=('rust' 'git')
source=("git+https://github.com/a-liashenko/bwsandbox.git")
sha256sums=('SKIP')

pkgver() {
    cd "$pkgname"
    printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
    cd "$pkgname"
    cargo build --release
}

package() {
    cd "$pkgname"
    install -Dm755 target/release/bwsandbox "$pkgdir/usr/bin/bwsandbox"
}
