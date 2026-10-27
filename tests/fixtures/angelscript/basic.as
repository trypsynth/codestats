// expect: lang="AngelScript" total=15 code=11 comment=2 blank=2 shebang=0
// Simple AngelScript game logic.
shared class Player
{
	uint8 lives = 3;
	int64 score = 0;
	array<string> items;

	void addItem(const string &in name)
	{
		items.insertLast(name);
	}
}

funcdef void Callback(Player@);
