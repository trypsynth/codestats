# expect: lang="BitBake" total=16 code=11 comment=2 blank=3 shebang=0
# Recipe for the hello example application.
SUMMARY = "Hello world example"
DESCRIPTION = "A minimal hello world application"
LICENSE = "MIT"
LIC_FILES_CHKSUM = "file://LICENSE;md5=0835ade698e0bcf8506ecda2f7b4f302"

SRC_URI = "file://hello.c"
DEPENDS = "zlib"

inherit autotools

do_install() {
	install -d ${D}${bindir}
	install -m 0755 hello ${D}${bindir}
}
