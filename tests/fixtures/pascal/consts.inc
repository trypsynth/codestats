// expect: lang="Pascal" total=12 code=8 comment=3 blank=1 shebang=0
{ Shared constants for the application. }
const
  MaxItems = 100;
  AppName = 'Inventory';

// Item record used across units.
type
  TItem = record
    Name: string;
    Quantity: Integer;
  end;
