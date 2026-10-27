#!/usr/bin/env bash
# expect: total=8 code=4 comment=2 blank=1 shebang=1
# small bash script
echo "hello"

if [ -f "file" ]; then
  echo "exists"
fi
