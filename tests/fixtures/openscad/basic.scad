// expect: total=10 code=5 comment=2 blank=3 shebang=0
// simple openscad fixture

module cube_box(size) {
	cube(size);
}

cube_box(2);

translate([1,1,1]) cube(1);
