#!/usr/bin/env bash
# expect: total=9 code=4 comment=2 blank=2 shebang=1
# small bash script
echo "hello"

if [ -f "file" ]; then
  echo "exists"
fi
