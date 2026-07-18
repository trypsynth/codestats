// expect: total=10 code=5 comment=2 blank=3 shebang=0
// tsx component fixture

import React from "react";

type Props = { name: string };

export function Hello(props: Props) {
	return <div>Hello {props.name}</div>;
}
