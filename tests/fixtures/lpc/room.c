// expect: lang="LPC" total=16 code=11 comment=2 blank=3 shebang=0
// A simple room in an LPC mudlib.
inherit "/std/room";

mapping exits;
mixed *items;

void create() {
	::create();
	set_short("A dusty hallway");
	exits = ([ "north": "/d/town/square" ]);
}

nomask varargs void init(object player) {
	add_action("do_search", "search");
}
