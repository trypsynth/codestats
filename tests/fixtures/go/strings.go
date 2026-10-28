// expect: lang="Go" total=8 code=6 comment=1 blank=1 shebang=0
package main

var glob = "pkg/**/*.go"
var raw = `first
/* not a comment
// nor this`
var r = '"'
