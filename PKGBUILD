pkgname=bwsandbox
pkgver=1.0.0
pkgrel=1
pkgdesc="Simple helper to run applications with seccomp and xdg-dbus-proxy via bubblewrap sandbox"
arch=('x86_64')
url="https://github.com/a-liashenko/bwsandbox"
license=('unknown')
depends=('bubblewrap' 'libseccomp')
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
