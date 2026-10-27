# expect: lang="BitBake" total=9 code=6 comment=2 blank=1 shebang=0
# Settings shared by the example recipes.
LICENSE = "MIT"
DEPENDS += "openssl"
SRC_URI += "file://defaults.conf"

do_compile:append() {
	oe_runmake extras
}
